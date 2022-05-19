use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Clone)]
pub struct ResourceManager {
    storage: Rc<RefCell<HashMap<TypeId, Box<dyn Any>>>>,
}

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

    #[must_use]
    pub fn insert<T>(&self, data: T) -> ResourceHandle<T> {
        let type_id = TypeId::of::<T>();
        let mut storage = self.storage.borrow_mut();
        unsafe {
            // SAFETY: We know that the type is correct.
            let vec = storage
                .entry(type_id)
                .or_insert_with(|| Box::new(Vec::<T>::with_capacity(1)))
                .downcast_mut::<Vec<T>>()
                .unwrap_unchecked();
            vec.push(data);

            // SAFETY: We just pushed an element to the Vec.
            ResourceHandle {
                idx: NonZeroUsize::new_unchecked(vec.len()),
                _marker: PhantomData,
            }
        }
    }

    pub fn get<T>(&self, handle: ResourceHandle<T>) -> ResourceRef<T> {
        let type_id = TypeId::of::<T>();
        ResourceRef {
            inner: Ref::map(self.storage.borrow(), |e| {
                e.get(&type_id)
                    .and_then(|e| e.downcast_ref::<Vec<T>>())
                    .and_then(|e| e.get(handle.idx.get() - 1))
                    .unwrap()
            }),
        }
    }
}
