use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::collections::HashMap;

use fugu::Context;

use crate::graphics::Graphics;
use crate::input::Input;

mod glutin;
mod sdl;

#[cfg(feature = "glutin")]
use self::glutin as backend;
#[cfg(feature = "sdl")]
use self::sdl as backend;

struct AbortOnDrop;

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        std::process::abort();
    }
}

fn replace_with<T, F: FnOnce(T) -> T>(dest: &mut T, f: F) {
    unsafe {
        let old = std::ptr::read(dest);
        let abort = AbortOnDrop;
        let new = f(old);
        std::mem::forget(abort);
        std::ptr::write(dest, new);
    }
}

fn type_name<T>() -> &'static str {
    let s = std::any::type_name::<T>();
    &s[s.rmatch_indices("::")
        .find_map(|(j, _)| (s.find('<').unwrap_or(s.len()) > j).then(|| j + 2))
        .unwrap_or(0)..]
}

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
    inner: HashMap<TypeId, UnsafeCell<Box<dyn Any>>>,
}

impl TypeMap {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn insert<T: 'static>(&mut self, v: T) {
        self.inner
            .insert(TypeId::of::<T>(), UnsafeCell::new(Box::new(v)));
    }

    unsafe fn get<T: 'static>(&self) -> Option<&T> {
        self.inner.get(&TypeId::of::<T>()).map(|v| {
            v.get()
                .as_ref()
                .unwrap_unchecked()
                .downcast_ref::<T>()
                .unwrap_unchecked()
        })
    }

    unsafe fn get_mut<T: 'static>(&self) -> Option<&mut T> {
        self.inner.get(&TypeId::of::<T>()).map(|v| {
            v.get()
                .as_mut()
                .unwrap_unchecked()
                .downcast_mut::<T>()
                .unwrap_unchecked()
        })
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

impl App {
    pub fn new() -> App {
        App {
            title: "Pufferfish".into(),
            size: (800, 600),
            vsync: true,
            resizable: true,
            state: TypeMap::new(),
            callbacks: Box::new(|_| {}),
        }
    }

    pub fn with_title(mut self, title: impl Into<Cow<'static, str>>) -> App {
        self.title = title.into();
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> App {
        self.size = (width, height);
        self
    }

    pub fn with_vsync(mut self, vsync: bool) -> App {
        self.vsync = vsync;
        self
    }

    pub fn with_resizable(mut self, resizable: bool) -> App {
        self.resizable = resizable;
        self
    }

    pub fn add_state<T: 'static>(mut self, state: T) -> App {
        self.state.insert(state);
        self
    }

    pub fn add_callback<'a, Args, T: Callback<Args> + 'static>(mut self, callback: T) -> App {
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

    fn init(&mut self, ctx: Context) {
        self.state.insert(Graphics::new(ctx));
        self.state.insert(Input::new());
    }
}
