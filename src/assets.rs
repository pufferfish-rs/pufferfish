//! Types relating to resource management and asset loading.
//!
//! All resources are backed by the [`ResourceManager`], which gives out
//! [`ResourceHandle`]s to be used to access different resources of various
//! types.
//!
//! The underlying resources can be accessed through the [`get`] method on
//! [`ResourceManager`]. The returned [`ResourceRef`] uniquely borrows the
//! underlying resource. This means that calling [`get`] while already holding
//! on to a [`ResourceRef`] to the same resource will result in a runtime panic.
//!
//! Therefore, the [`ResourceRef`] *should not* be held on to for
//! longer than necessary. The [`ResourceHandle`] should be kept around instead,
//! and the [`ResourceRef`] should be reacquired each time it is needed.
//!
//! To make this easier, the internal storage of the [`ResourceManager`] is
//! wrapped inside a [`Rc`]. This means cloning the [`ResourceManager`] is cheap
//! and will return a new [`ResourceManager`] with the same internal storage.
//! You are encouraged to keep around a copy if you need to access resources
//! frequently.
//!
//! The restrictions described above may be relaxed in the future.
//!
//! [`get`]: ResourceManager::get

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::{RefCell, UnsafeCell};
use std::collections::{BTreeMap, HashMap};
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::rc::Rc;

use self::fs::BasicFileSystem;
use crate::experimental::{FileSystem, FileTask};

pub mod fs;

/// Central storage of resources used in the game. Accessible from
/// [`App`](crate::App) by default.
///
/// For loading assets, you probably do not want to use this directly. In most
/// cases, [Assets] should be used instead.
///
/// Cloning a [`ResourceManager`] will do a shallow copy, meaning copies will
/// all use the same internal storage. You do not have to and should not wrap
/// this type in an `Rc`.
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: self
#[derive(Clone)]
pub struct ResourceManager {
    storage: Rc<RefCell<BTreeMap<(TypeId, u64), Resource>>>,
}

/// A handle to a resource of type `T`.
///
/// A `ResourceHandle` can be acquired directly through the [`allocate`] method
/// on [`ResourceManager`] or indirectly through the [`load`] method on
/// [`Assets`]. The backing resource of type `T` can be accessed through the
/// [`get`] method on [`ResourceManager`].
///
/// An `Option<ResourceHandle<T>>` is guaranteed to be the same size as a bare
/// `ResourceHandle<T>`.
///
/// [`allocate`]: ResourceManager::allocate
/// [`load`]: Assets::load
/// [`get`]: ResourceManager::get
pub struct ResourceHandle<T: 'static> {
    idx: NonZeroU64,
    _marker: PhantomData<*const T>,
}

impl<T: 'static> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for ResourceHandle<T> {}

impl<T: 'static> PartialEq for ResourceHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T: 'static> Eq for ResourceHandle<T> {}

impl<T: 'static> Hash for ResourceHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state);
    }
}

impl<T: 'static> Debug for ResourceHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(crate::util::type_name::<ResourceHandle<T>>())
            .field("idx", &self.idx)
            .finish()
    }
}

fn transmute_handle<T, U>(handle: ResourceHandle<T>) -> ResourceHandle<U> {
    ResourceHandle {
        idx: handle.idx,
        _marker: PhantomData,
    }
}

#[derive(Clone)]
struct Resource(Rc<UnsafeCell<dyn Any>>);

#[repr(C)]
struct ResourceInner<T> {
    lock: bool,
    data: T,
}

impl Resource {
    fn new<T: 'static>(data: T) -> Self {
        Resource(Rc::new(UnsafeCell::new(ResourceInner {
            lock: false,
            data,
        })))
    }

    fn lock(&self) {
        unsafe {
            let inner = self.0.get() as *mut bool;
            if *inner {
                panic!("cannot acquire lock to resource that is already locked");
            }
            *inner = true;
        }
    }

    fn unlock(&self) {
        unsafe {
            let inner = self.0.get() as *mut bool;
            *inner = false;
        }
    }

    unsafe fn downcast_ref<T>(&self) -> &T {
        let inner = self.0.get() as *const dyn Any as *const ResourceInner<T>;
        &(*inner).data
    }

    #[allow(clippy::mut_from_ref)]
    unsafe fn downcast_mut<T>(&self) -> &mut T {
        let inner = self.0.get() as *mut ResourceInner<T>;
        &mut (*inner).data
    }
}

/// A reference to a resource of type `T`.
///
/// Generally speaking, *do not* hold on to a `ResourceRef` for longer than
/// necessary, as attempting to acquire a new `ResourceRef` while already
/// holding one to the same resource will result in a panic. Instead, you should
/// hold on to the [`ResourceHandle`] and reacquire the `ResourceRef` each time
/// you need it.
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: self
pub struct ResourceRef<T> {
    inner: Resource,
    _marker: PhantomData<T>,
}

impl<T> ResourceRef<T> {
    fn new(inner: Resource) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<T> Deref for ResourceRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe {
            self.inner
                .downcast_ref::<Option<T>>()
                .as_ref()
                .unwrap_unchecked()
        }
    }
}

impl<T> DerefMut for ResourceRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            self.inner
                .downcast_mut::<Option<T>>()
                .as_mut()
                .unwrap_unchecked()
        }
    }
}

impl<T> Drop for ResourceRef<T> {
    fn drop(&mut self) {
        self.inner.unlock();
    }
}

impl ResourceManager {
    pub(crate) fn new() -> Self {
        Self {
            storage: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }

    /// Allocates and returns a new [`ResourceHandle`] for the given type.
    ///
    /// See the [module-level documentation] for more information.
    ///
    /// [module-level documentation]: self
    #[must_use]
    pub fn allocate<T>(&self) -> ResourceHandle<T> {
        let type_id = TypeId::of::<T>();
        let mut storage = self.storage.borrow_mut();

        // Find the greatest index and add 1 (or use 1 if there is no index).
        let idx = storage
            .keys()
            .rfind(|k| k.0 == type_id)
            .map(|e| e.1 + 1)
            .unwrap_or(1);

        storage.insert((type_id, idx), Resource::new::<Option<T>>(None));

        // SAFETY: idx cannot be zero.
        ResourceHandle {
            idx: unsafe { NonZeroU64::new_unchecked(idx) },
            _marker: PhantomData,
        }
    }

    /// Sets the underlying value of the given [`ResourceHandle`].
    ///
    /// See the [module-level documentation] for more information.
    ///
    /// [module-level documentation]: self
    pub fn set<T>(&self, handle: ResourceHandle<T>, data: T) {
        let type_id = TypeId::of::<T>();
        let mut storage = self.storage.borrow_mut();
        unsafe {
            // SAFETY: We know everything exists and the type is correct.
            let resource = storage
                .get_mut(&(type_id, handle.idx.get()))
                .unwrap_unchecked();
            resource.lock();
            *resource.downcast_mut::<Option<T>>() = Some(data);
            resource.unlock();
        }
    }

    /// Returns a reference to the underlying value of the given
    /// [`ResourceHandle`].
    ///
    /// Generally speaking, you should call this function again every time you
    /// need to access the data instead of keeping around the returned
    /// guard.
    ///
    /// # Panics
    ///
    /// Panics if there is already a [`ResourceRef`] to the same resource.
    ///
    /// See the [module-level documentation] for more information.
    ///
    /// [module-level documentation]: self
    pub fn get<T>(&self, handle: ResourceHandle<T>) -> Option<ResourceRef<T>> {
        let type_id = TypeId::of::<T>();
        unsafe {
            let mut inner = self.storage.borrow_mut();
            let inner = inner
                .get_mut(&(type_id, handle.idx.get()))
                .unwrap_unchecked();
            inner.lock();
            if inner.downcast_ref::<Option<T>>().is_none() {
                inner.unlock();
                None
            } else {
                Some(ResourceRef::new(inner.clone()))
            }
        }
    }
}

/// Abstraction for loading assets. Accessible from [`App`](crate::App) by
/// default.
///
/// By default, asset files are loaded from a [`BasicFileSystem`] initialized
/// with default settings. To change this behavior, set the file system by
/// calling [`set_fs`]. Also note that the default file system is not allocated
/// and initialized until you attempt to read a file.
///
/// [`set_fs`]: Self::set_fs
/// [`init_fs`]: Self::init_fs
pub struct Assets {
    resource_manager: ResourceManager,
    fs: Box<dyn FileSystem>,
    fs_init: bool,
    loaders: HashMap<(TypeId, Cow<'static, str>), Loader>,
    handles: HashMap<(TypeId, Cow<'static, str>), ResourceHandle<()>>,
    tasks: Vec<FileTaskResolve>,
}

type Loader = Rc<dyn Fn(&[u8], &mut Resource)>;

impl Assets {
    pub(crate) fn new(resource_manager: &ResourceManager) -> Self {
        struct PlaceholderFileSystem;
        impl FileSystem for PlaceholderFileSystem {
            fn read(&mut self, _: &Path) -> Box<dyn FileTask> {
                unreachable!()
            }
        }

        Self {
            resource_manager: resource_manager.clone(),
            fs: Box::new(PlaceholderFileSystem),
            fs_init: false,
            loaders: HashMap::new(),
            handles: HashMap::new(),
            tasks: Vec::new(),
        }
    }

    /// Registers a loader for the given type.
    ///
    /// # Arguments
    ///
    /// * `extensions` - An array of file extensions to apply the loader to.
    /// * `loader` - A closure that takes a byte slice and returns a value of
    ///   type `T`.
    pub fn add_loader<T: 'static, const LEN: usize>(
        &mut self,
        extensions: [impl Into<Cow<'static, str>>; LEN],
        loader: impl Fn(&[u8]) -> T + 'static,
    ) {
        let loader: Rc<dyn Fn(&[u8], &mut Resource)> =
            Rc::new(move |data: &[u8], resource: &mut Resource| {
                let val = loader(data);
                resource.lock();
                // SAFETY: We know that the type is correct and we have the lock.
                unsafe {
                    *resource.downcast_mut::<Option<T>>() = Some(val);
                }
                resource.unlock();
            });
        for extension in extensions {
            self.loaders
                .insert((TypeId::of::<T>(), extension.into()), Rc::clone(&loader));
        }
    }

    /// Sets the file system to use for loading assets.
    pub fn set_fs(&mut self, fs: impl FileSystem + 'static) {
        self.fs = Box::new(fs);
        self.fs_init = true;
    }

    /// Returns a [`ResourceHandle`] of the given type representing the asset at
    /// the given path. The loader is only called and a new [`ResourceHandle`]
    /// allocated if the asset has not already been loaded. Otherwise, a copy of
    /// the existing handle is returned.
    ///
    /// Assets are not guaranteed to have been loaded by the time this function
    /// returns, so you should gracefully handle cases where the asset is not
    /// loaded yet.
    ///
    /// # Panics
    ///
    /// Panics if no asset exists at the given path, the asset cannot be loaded
    /// successfully, or no loader matches the given file extension and type.
    pub fn load<T: 'static>(&mut self, path: impl Into<Cow<'static, str>>) -> ResourceHandle<T> {
        let type_id = TypeId::of::<T>();
        let path: Cow<'static, str> = path.into();

        transmute_handle(
            *self
                .handles
                .entry((type_id, path.clone()))
                .or_insert_with(|| {
                    let path: &str = &path;
                    let path = Path::new(path);
                    let handle = self.resource_manager.allocate::<T>();
                    if !self.fs_init {
                        self.fs = Box::new(BasicFileSystem::new());
                        self.fs_init = true;
                    }
                    let mut task = FileTaskResolve {
                        task: self.fs.read(path),
                        type_id,
                        idx: handle.idx,
                    };
                    if !task.poll(&mut self.loaders, &self.resource_manager) {
                        self.tasks.push(task);
                    }
                    transmute_handle(handle)
                }),
        )
    }

    /// Updates any pending file loads. This is called internally at the start
    /// of each frame.
    pub fn update(&mut self) {
        let mut i = 0;
        while i < self.tasks.len() {
            if self.tasks[i].poll(&mut self.loaders, &self.resource_manager) {
                self.tasks.remove(i);
            } else {
                i += 1;
            }
        }
    }
}

struct FileTaskResolve {
    task: Box<dyn FileTask>,
    type_id: TypeId,
    idx: NonZeroU64,
}

impl FileTaskResolve {
    fn poll(
        &mut self,
        loaders: &mut HashMap<(TypeId, Cow<'static, str>), Loader>,
        rm: &ResourceManager,
    ) -> bool {
        let complete = self.task.poll();
        if complete {
            let loader = loaders
                .get(&(
                    self.type_id,
                    self.task
                        .path()
                        .extension()
                        .and_then(|e| e.to_str())
                        .unwrap()
                        .into(),
                ))
                .unwrap();
            let mut storage = rm.storage.borrow_mut();
            let resource = storage.get_mut(&(self.type_id, self.idx.get())).unwrap();
            loader(self.task.data(), resource);
        }
        complete
    }
}
