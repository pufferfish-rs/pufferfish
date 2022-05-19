use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ptr::NonNull;

use fugu::Context;

use crate::assets::ResourceManager;
use crate::graphics::Graphics;
use crate::input::Input;
use crate::util::{replace_with, type_name};

mod sdl;
use self::sdl as backend;

struct ArgDesc {
    tid: TypeId,
    tname: &'static str,
    unique: bool,
}

trait Argable {
    unsafe fn get(args: *mut TypeMap) -> Self;
    fn desc() -> ArgDesc;
}

impl<T: 'static> Argable for &T {
    unsafe fn get(args: *mut TypeMap) -> Self {
        args.as_mut().unwrap_unchecked().get::<T>().unwrap()
    }

    fn desc() -> ArgDesc {
        ArgDesc {
            tid: TypeId::of::<T>(),
            tname: type_name::<T>(),
            unique: false,
        }
    }
}

impl<T: 'static> Argable for &mut T {
    unsafe fn get(args: *mut TypeMap) -> Self {
        args.as_mut().unwrap_unchecked().get_mut::<T>().unwrap()
    }

    fn desc() -> ArgDesc {
        ArgDesc {
            tid: TypeId::of::<T>(),
            tname: type_name::<T>(),
            unique: true,
        }
    }
}

pub struct TypeMap {
    inner: HashMap<TypeId, NonNull<dyn Any>>,
}

impl TypeMap {
    fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    fn insert<T: 'static>(&mut self, v: T) {
        let type_id = TypeId::of::<T>();
        if let Some(v) = self.inner.remove(&type_id) {
            unsafe {
                Box::from_raw(v.as_ptr());
            }
        }
        // SAFETY: The pointer returned by Box::into_raw is guaranteed to be non-null.
        self.inner.insert(TypeId::of::<T>(), unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(v)))
        });
    }

    /// # Safety
    ///
    /// Aliasing rules must be enforced by the caller.
    unsafe fn get<T: 'static>(&self) -> Option<&T> {
        self.inner.get(&TypeId::of::<T>()).map(|v| {
            // SAFETY: The types are guaranteed to match.
            v.as_ref().downcast_ref::<T>().unwrap_unchecked()
        })
    }

    /// # Safety
    ///
    /// Aliasing rules must be enforced by the caller.
    unsafe fn get_mut<T: 'static>(&self) -> Option<&mut T> {
        self.inner.get(&TypeId::of::<T>()).map(|v| {
            // SAFETY: The pointer is guaranteed to be non-null with matching types.
            v.as_ptr()
                .as_mut()
                .and_then(|e| e.downcast_mut::<T>())
                .unwrap_unchecked()
        })
    }
}

impl Drop for TypeMap {
    fn drop(&mut self) {
        for (_, v) in self.inner.drain() {
            unsafe {
                Box::from_raw(v.as_ptr());
            }
        }
    }
}

pub trait Callback<Args> {
    fn call(&self, args: &mut TypeMap);
    fn assert_legal();
}

macro_rules! impl_callback {
    ($($first:ident$(, $($other:ident),+)?$(,)?)?) => {
        impl<$($first$(, $($other),+)?,)? Func> Callback<($($first$(, $($other),+)?)?,)> for Func where Func: Fn($($first$(, $($other),+)?,)?), $($first: Argable$(, $($other: Argable),+)?,)? {
            fn call(&self, args: &mut TypeMap) {
                // SAFETY: We already asserted that the callback signature is legal.
                unsafe { self($($first::get(args)$(, $($other::get(args)),+)?,)?) }
            }

            fn assert_legal() {
                let arg_types = &[$($first::desc()$(, $($other::desc()),+)?,)?];
                for (i, a) in arg_types.iter().enumerate() {
                    for (j, b) in arg_types.iter().enumerate() {
                        if i != j && a.tid == b.tid {
                            if a.unique && b.unique {
                                panic!("illegal callback signature ({}): multiple unique references to {}", arg_types.iter().flat_map(|e| [", ", if e.unique { "&mut " } else { "&" }, e.tname]).skip(1).collect::<String>(), a.tname);
                            } else if a.unique || b.unique {
                                panic!("illegal callback signature ({}): both unique and shared references to {}", arg_types.iter().flat_map(|e| [", ", if e.unique { "&mut " } else { "&" }, e.tname]).skip(1).collect::<String>(), a.tname);
                            }
                        }
                    }
                }
            }
        }
        $($(impl_callback!($($other),+);)?)?
    };
}

impl_callback!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

pub struct App {
    title: Cow<'static, str>,
    size: (u32, u32),
    vsync: bool,
    resizable: bool,
    state: TypeMap,
    callbacks: Box<dyn Fn(&mut TypeMap)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            title: "Pufferfish".into(),
            size: (800, 600),
            vsync: true,
            resizable: true,
            state: TypeMap::new(),
            callbacks: Box::new(|_| {}),
        }
    }
}

impl App {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    pub fn add_state<T: 'static>(mut self, state: T) -> Self {
        self.state.insert(state);
        self
    }

    pub fn add_callback<Args, T: Callback<Args> + 'static>(mut self, callback: T) -> Self {
        T::assert_legal();
        replace_with(&mut self.callbacks, |cbs| {
            Box::new(move |args| {
                cbs(args);
                callback.call(args);
            })
        });
        self
    }

    pub fn run(self) {
        backend::run(self);
    }

    fn init(&mut self, ctx: Context, resource_manager: &ResourceManager) {
        self.state.insert(resource_manager.clone());
        self.state.insert(Graphics::new(ctx, resource_manager));
        self.state.insert(Input::new());
    }
}
