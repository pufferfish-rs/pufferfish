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
use std::cell::{RefCell, UnsafeCell};
use std::collections::BTreeMap;
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
/// `ResourceHandle<T>`. Otherwise, the representation of this type is
/// unspecified and may change at any time.
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

impl<T: 'static> Hash for ResourceHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.idx.get());
    }
}

impl<T: 'static> Debug for ResourceHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ResourceHandle<{}>({})",
            std::any::type_name::<T>(),
            self.idx.get()
        )
    }
}

impl<T: 'static> ResourceHandle<T> {
    /// Returns the `ResourceId` corresponding to this handle.
    pub fn to_id(self) -> ResourceId {
        ResourceId { idx: self.idx }
    }

    /// Creates a `ResourceHandle` from a `ResourceId`.
    ///
    /// # Safety
    ///
    /// This may produce invalid `ResourceHandle`s if the `ResourceId` was
    /// created from a `ResourceHandle` of a different type. It is undefined
    /// behavior to use an invalid `ResourceHandle`.
    pub unsafe fn from_id(id: ResourceId) -> Self {
        ResourceHandle {
            idx: id.idx,
            _marker: PhantomData,
        }
    }
}

/// A raw identifier for a resource.
///
/// An `Option<ResourceId>` is guaranteed to be the same size as a bare
/// `ResourceId`. Otherwise, the representation of this type is unspecified and
/// may change at any time.
///
/// In most cases, you should use [`ResourceHandle`] instead.
pub struct ResourceId {
    idx: NonZeroU64,
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
