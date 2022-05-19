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

pub struct Assets {
    resource_manager: ResourceManager,
    loaders: HashMap<TypeId, Box<dyn Any>>,
    handles: HashMap<TypeId, Box<dyn Any>>,
}

impl Assets {
    pub(crate) fn new(resource_manager: &ResourceManager) -> Self {
        Self {
            resource_manager: resource_manager.clone(),
            loaders: HashMap::new(),
            handles: HashMap::new(),
        }
    }

    pub fn add_loader<T: 'static>(
        &mut self,
        extension: impl Into<Cow<'static, str>>,
        loader: impl Fn(&[u8]) -> T + 'static,
    ) {
        let loaders = unsafe {
            self.loaders
                .entry(TypeId::of::<T>())
                .or_insert_with(|| {
                    Box::new(HashMap::<Cow<'static, str>, Box<dyn Fn(&[u8]) -> T>>::new())
                })
                .downcast_mut::<HashMap<Cow<'static, str>, Box<dyn Fn(&[u8]) -> T>>>()
                .unwrap_unchecked()
        };
        loaders.insert(extension.into(), Box::new(loader));
    }

    pub fn load<T: 'static>(&mut self, path: impl Into<Cow<'static, str>>) -> ResourceHandle<T> {
        use std::fs::read;
        use std::path::Path;

        let type_id = TypeId::of::<T>();
        let path: Cow<'static, str> = path.into();

        *unsafe {
            self.handles
                .entry(type_id)
                .or_insert_with(|| Box::new(HashMap::<Cow<'static, str>, ResourceHandle<T>>::new()))
                .downcast_mut::<HashMap<Cow<'static, str>, ResourceHandle<T>>>()
                .unwrap_unchecked()
        }
        .entry(path.clone())
        .or_insert_with(|| {
            let path: &str = &path;
            let path = Path::new(path);
            let ext = path.extension().and_then(|e| e.to_str()).unwrap();
            let loader = self
                .loaders
                .get(&type_id)
                .and_then(|e| e.downcast_ref::<HashMap<Cow<str>, Box<dyn Fn(&[u8]) -> T>>>())
                .and_then(|e| e.get(ext))
                .unwrap();
            let handle = self.resource_manager.allocate();
            self.resource_manager
                .set(handle, loader(&read(path).unwrap()));
            handle
        })
    }
}
