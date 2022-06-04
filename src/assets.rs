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
use std::rc::Rc;

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
        inner.lock();
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
            let val = storage
                .get_mut(&(type_id, handle.idx.get()))
                .unwrap_unchecked()
                .downcast_mut::<Option<T>>();
            *val = Some(data);
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
            if inner.downcast_ref::<Option<T>>().is_none() {
                None
            } else {
                Some(ResourceRef::new(inner.clone()))
            }
        }
    }
}

/// Abstraction for loading assets. Accessible from [`App`](crate::App) by
/// default.
pub struct Assets {
    resource_manager: ResourceManager,
    loaders: HashMap<(TypeId, Cow<'static, str>), Box<dyn Any>>,
    handles: HashMap<(TypeId, Cow<'static, str>), ResourceHandle<()>>,
}

impl Assets {
    pub(crate) fn new(resource_manager: &ResourceManager) -> Self {
        Self {
            resource_manager: resource_manager.clone(),
            loaders: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    /// Registers a loader for the given type.
    ///
    /// # Arguments
    ///
    /// * `extension` - The file extension to apply the loader to.
    /// * `loader` - A closure that takes a byte slice and returns a value of
    ///   type `T`.
    pub fn add_loader<T: 'static>(
        &mut self,
        extension: impl Into<Cow<'static, str>>,
        loader: impl Fn(&[u8]) -> T + 'static,
    ) {
        self.loaders.insert(
            (TypeId::of::<T>(), extension.into()),
            Box::<Box<dyn Fn(&[u8]) -> T>>::new(Box::new(loader)),
        );
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
        use std::fs::read;
        use std::path::Path;

        let type_id = TypeId::of::<T>();
        let path: Cow<'static, str> = path.into();

        transmute_handle(
            *self
                .handles
                .entry((type_id, path.clone()))
                .or_insert_with(|| {
                    let path: &str = &path;
                    let path = Path::new(path);
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap();
                    let loader = self
                        .loaders
                        .get(&(type_id, ext.into()))
                        .and_then(|e| e.downcast_ref::<Box<dyn Fn(&[u8]) -> T + 'static>>())
                        .unwrap();
                    let handle = self.resource_manager.allocate();
                    self.resource_manager
                        .set(handle, loader(&read(path).unwrap()));
                    transmute_handle(handle)
                }),
        )
    }
}
