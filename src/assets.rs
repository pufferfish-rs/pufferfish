//! Types relating to resource management and asset loading.
//!
//! All resources are backed by the [`ResourceManager`], which gives out
//! [`ResourceHandle`]s to be used to access different resources of various
//! types.
//!
//! The underlying resources can be accessed through the [`get`] method on
//! [`ResourceManager`]. However, currently, the returned [`ResourceRef`]
//! borrows the entire storage. This means that as long as a [`ResourceRef`] is
//! alive, calling [`ResourceManager::allocate`] or calling
//! [`ResourceManager::set`] on any resource handle will result in a runtime
//! panic.
//!
//! Therefore, the [`ResourceRef`] **should not** be held on to for
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
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::Deref;
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
    storage: Rc<RefCell<HashMap<TypeId, Box<dyn Any>>>>,
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
    idx: NonZeroUsize,
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

/// A reference to a resource of type `T`.
///
/// **Do not** hold on to a `ResourceRef`, as attempting to allocate new
/// [`ResourceHandle`]s or set previous ones while a `ResourceRef` is held will
/// currently result in a panic. Instead, you should hold onto the
/// [`ResourceHandle`] and reborrow the underlying data each time you need it.
///
/// See the [module-level documentation] for more information.
///
/// [module-level documentation]: self
pub struct ResourceRef<'a, T> {
    inner: Ref<'a, T>,
}

impl<T> Deref for ResourceRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ResourceManager {
    pub(crate) fn new() -> Self {
        Self {
            storage: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Allocates and returns a new [`ResourceHandle`] for the given type.
    ///
    /// # Panics
    ///
    /// Calling this function while holding on to a [`ResourceRef`], regardless
    /// of what resource it points to, will result in a panic.
    ///
    /// See the [module-level documentation] for more information.
    ///
    /// [module-level documentation]: self
    #[must_use]
    pub fn allocate<T>(&self) -> ResourceHandle<T> {
        let type_id = TypeId::of::<T>();
        let mut storage = self.storage.borrow_mut();
        unsafe {
            // SAFETY: We know that the type is correct.
            let vec = storage
                .entry(type_id)
                .or_insert_with(|| Box::new(Vec::<Option<T>>::with_capacity(1)))
                .downcast_mut::<Vec<Option<T>>>()
                .unwrap_unchecked();
            vec.push(None);

            // SAFETY: We just pushed an element to the Vec.
            ResourceHandle {
                idx: NonZeroUsize::new_unchecked(vec.len()),
                _marker: PhantomData,
            }
        }
    }

    /// Sets the underlying value of the given [`ResourceHandle`].
    ///
    /// # Panics
    ///
    /// Calling this function while holding on to a [`ResourceRef`], regardless
    /// of what resource it points to, will result in a panic.
    ///
    /// See the [module-level documentation] for more information.
    ///
    /// [module-level documentation]: self
    pub fn set<T>(&self, handle: ResourceHandle<T>, data: T) {
        let type_id = TypeId::of::<T>();
        let mut storage = self.storage.borrow_mut();
        unsafe {
            // SAFETY: We know everything exists and the type is correct.
            let vec = storage
                .get_mut(&type_id)
                .and_then(|e| e.downcast_mut::<Vec<Option<T>>>())
                .unwrap_unchecked();
            vec[handle.idx.get() - 1] = Some(data);
        }
    }

    /// Returns a reference to the underlying value of the given
    /// [`ResourceHandle`].
    ///
    /// You should call this function again every time you need to access the
    /// data instead of keeping around the returned reference.
    ///
    /// See the [module-level documentation] for more information.
    ///
    /// [module-level documentation]: self
    pub fn get<T>(&self, handle: ResourceHandle<T>) -> Option<ResourceRef<T>> {
        let type_id = TypeId::of::<T>();
        let inner = Ref::map(self.storage.borrow(), |e| {
            e.get(&type_id)
                .and_then(|e| e.downcast_ref::<Vec<Option<T>>>())
                .and_then(|e| e.get(handle.idx.get() - 1))
                .unwrap()
        });
        if inner.is_none() {
            None
        } else {
            Some(ResourceRef {
                inner: Ref::map(inner, |e| e.as_ref().unwrap()),
            })
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
