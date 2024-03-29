//! Types relating to resource management and asset loading.
//!
//! All resources are backed by the [`ResourceManager`], which gives out
//! [`ResourceHandle`]s to be used to access different resources of various
//! types.
//!
//! The underlying resources can be accessed through the [`get`] and [`get_mut`]
//! methods on [`ResourceManager`]. Borrowing rules are dynamically enforced on
//! the returned locks to ensure validity. Any violation of these rules will
//! result in a panic.
//!
//! Therefore, these locks generally *should not* be held on to for longer than
//! necessary. The [`ResourceHandle`] should be kept around instead,
//! and resources should be reborrowed each time they are needed.
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
//! [`get_mut`]: ResourceManager::get_mut

use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::{RefCell, UnsafeCell};
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::{BuildHasher, Hash, Hasher};
use std::marker::PhantomData;
use std::num::NonZeroU64;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::rc::Rc;

use hashbrown::HashMap;

use self::fs::ThreadedFileSystem;
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
        f.debug_tuple(crate::util::type_name::<ResourceHandle<T>>())
            .field(&self.idx)
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
    flag: isize,
    data: T,
}

impl Resource {
    fn new<T: 'static>(data: T) -> Self {
        Resource(Rc::new(UnsafeCell::new(ResourceInner { flag: 0, data })))
    }

    fn lock(&self) {
        unsafe {
            let flag = self.0.get() as *mut isize;
            if *flag >= 0 {
                *flag += 1;
            } else {
                panic!("cannot immutably borrow resource; already mutably borrowed");
            }
        }
    }

    fn lock_mut(&self) {
        unsafe {
            let flag = self.0.get() as *mut isize;
            match *flag {
                0 => *flag = -1,
                x if x < 0 => panic!("cannot mutably borrow resource more than once"),
                _ => panic!("cannot mutably borrow resource; already immutably borrowed"),
            }
        }
    }

    fn unlock(&self) {
        unsafe {
            let flag = self.0.get() as *mut isize;
            debug_assert!(*flag > 0); // flag should be positive for immutable borrows
            *flag -= 1;
        }
    }

    fn unlock_mut(&self) {
        unsafe {
            let flag = self.0.get() as *mut isize;
            debug_assert_eq!(*flag, -1); // flag should be -1 for a mutable borrow
            *flag = 0;
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

/// An immutable reference to a resource of type `T`.
///
/// Attempting to borrow a resource mutably while holding a [`ResourceRef`]
/// to it will result in a panic. It is therefore generally recommended to store
/// the [`ResourceHandle`] instead, reborrowing the resource each time you
/// need it.
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

impl<T> Drop for ResourceRef<T> {
    fn drop(&mut self) {
        self.inner.unlock();
    }
}

/// A mutable reference to a resource of type `T`.
///
/// Attempting to borrow a resource while already holding a [`ResourceRefMut`]
/// to it will result in a panic. It is therefore generally recommended to store
/// the [`ResourceHandle`] instead, reborrowing the resource each time you
/// need it.
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: self
pub struct ResourceRefMut<T> {
    inner: Resource,
    _marker: PhantomData<T>,
}

impl<T> ResourceRefMut<T> {
    fn new(inner: Resource) -> Self {
        Self {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<T> Deref for ResourceRefMut<T> {
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

impl<T> DerefMut for ResourceRefMut<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            self.inner
                .downcast_mut::<Option<T>>()
                .as_mut()
                .unwrap_unchecked()
        }
    }
}

impl<T> Drop for ResourceRefMut<T> {
    fn drop(&mut self) {
        self.inner.unlock_mut();
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

    /// Sets the underlying resource corresponding to the given
    /// [`ResourceHandle`].
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

    /// Immutably borrows the underlying resource correspoding to the given
    /// [`ResourceHandle`].
    ///
    /// # Panics
    ///
    /// Panics if the given resource is currently mutably borrowed.
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

    /// Mutably borrows the underlying resource correspoding to the given
    /// [`ResourceHandle`].
    ///
    /// # Panics
    ///
    /// Panics if the given resource is currently borrowed.
    ///
    /// See the [module-level documentation] for more information.
    ///
    /// [module-level documentation]: self
    pub fn get_mut<T>(&self, handle: ResourceHandle<T>) -> Option<ResourceRefMut<T>> {
        let type_id = TypeId::of::<T>();
        unsafe {
            let mut inner = self.storage.borrow_mut();
            let inner = inner
                .get_mut(&(type_id, handle.idx.get()))
                .unwrap_unchecked();
            inner.lock_mut();
            if inner.downcast_ref::<Option<T>>().is_none() {
                inner.unlock_mut();
                None
            } else {
                Some(ResourceRefMut::new(inner.clone()))
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
    loaders: HashMap<(TypeId, Cow<'static, str>), Option<Loader>>,
    handles: HashMap<(TypeId, Cow<'static, str>), ResourceHandle<()>>,
    tasks: Vec<Option<FileTaskResolve>>,
}

type Loader = Rc<dyn Fn(&[u8], &mut Assets, TypeId, NonZeroU64)>;

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
        loader: impl Fn(&[u8], &mut Assets) -> T + 'static,
    ) {
        let loader: Loader = Rc::new(move |data, assets, type_id, idx| {
            let val = loader(data, assets);
            let mut storage = assets.resource_manager.storage.borrow_mut();
            let resource = storage.get_mut(&(type_id, idx.get())).unwrap();
            resource.lock();
            // SAFETY: We know that the type is correct and we have the lock.
            unsafe {
                *resource.downcast_mut::<Option<T>>() = Some(val);
            }
            resource.unlock();
        });
        for extension in extensions {
            self.loaders.insert(
                (TypeId::of::<T>(), extension.into()),
                Some(Rc::clone(&loader)),
            );
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

        let mut hasher = self.handles.hasher().build_hasher();
        (type_id, &path).hash(&mut hasher);
        let hash = hasher.finish();

        transmute_handle(
            if let Some((_, &handle)) = self
                .handles
                .raw_entry()
                .from_hash(hash, |(a, b)| a == &type_id && b == &path)
            {
                handle
            } else {
                let p = Path::new(&*path);
                let handle = self.resource_manager.allocate::<T>();
                if !self.fs_init {
                    self.fs = Box::new(ThreadedFileSystem::new());
                    self.fs_init = true;
                }
                let mut task = FileTaskResolve {
                    task: self.fs.read(p),
                    type_id,
                    idx: handle.idx,
                };
                if !task.poll(self) {
                    self.tasks.push(Some(task));
                }
                let handle = transmute_handle(handle);
                self.handles.insert((type_id, path), handle);
                handle
            },
        )
    }

    /// Updates any pending file loads. This is called internally at the start
    /// of each frame.
    pub fn update(&mut self) {
        let mut i = 0;
        while i < self.tasks.len() {
            let mut task = self.tasks[i].take().unwrap();
            if task.poll(self) {
                self.tasks.remove(i);
            } else {
                self.tasks[i] = Some(task);
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
    fn poll(&mut self, assets: &mut Assets) -> bool {
        let complete = self.task.poll();
        if complete {
            let key = (self.type_id, self.task.extension().to_owned().into());
            let loader = assets.loaders.get_mut(&key).unwrap();
            let loader = loader.take().unwrap();
            loader(self.task.data(), assets, self.type_id, self.idx);
            *assets.loaders.get_mut(&key).unwrap() = Some(loader);
        }
        complete
    }
}
