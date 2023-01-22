use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::ptr::NonNull;
use std::rc::Rc;

use fugu::Context;
use hashbrown::HashMap;

use crate::assets::{Assets, ResourceManager};
use crate::graphics::{Graphics, Sprite};
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

/// A heterogeneous collection that can store one value of each type.
#[derive(Default)]
pub struct TypeMap {
    inner: HashMap<TypeId, NonNull<dyn Any>>,
}

impl TypeMap {
    /// Creates an empty `TypeMap`.
    ///
    /// The type map is initially created with a capacity of 0, so it will not
    /// allocate until it is first inserted into.
    pub fn new() -> Self {
        Default::default()
    }

    /// Inserts a value of type `T` into the type map, replacing the previous
    /// value if it already exists.
    pub fn insert<T: 'static>(&mut self, v: T) {
        let type_id = TypeId::of::<T>();
        if let Some(v) = self.inner.remove(&type_id) {
            unsafe {
                drop(Box::from_raw(v.as_ptr()));
            }
        }
        // SAFETY: The pointer returned by Box::into_raw is guaranteed to be non-null.
        self.inner.insert(TypeId::of::<T>(), unsafe {
            NonNull::new_unchecked(Box::into_raw(Box::new(v)))
        });
    }

    /// Returns a reference to the value of type `T` if it exists.
    ///
    /// # Safety
    ///
    /// Aliasing rules must be enforced by the caller.
    pub unsafe fn get<T: 'static>(&self) -> Option<&T> {
        self.inner.get(&TypeId::of::<T>()).map(|v| {
            // SAFETY: The types are guaranteed to match.
            v.as_ref().downcast_ref::<T>().unwrap_unchecked()
        })
    }

    /// Returns a mutable reference to the value of type `T` if it exists.
    ///
    /// # Safety
    ///
    /// Aliasing rules must be enforced by the caller.
    pub unsafe fn get_mut<T: 'static>(&self) -> Option<&mut T> {
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
                drop(Box::from_raw(v.as_ptr()));
            }
        }
    }
}

/// An interface for callbacks.
///
/// Callbacks may borrow arbitrary state from the application through their type
/// signature.
pub trait Callback<Args, Output> {
    /// Calls the callback with the given state and returns its output.
    ///
    /// # Safety
    /// It is up to the caller to guarantee that the callback's type signature
    /// is legal. Calling this function on an illegal callback is undefined
    /// behavior.
    unsafe fn call(&self, args: &mut TypeMap) -> Output;

    /// Asserts that the callback's type signature is legal.
    ///
    /// # Panics
    /// Panics if the type signature of the callback violates aliasing rules.
    fn assert_legal();
}

macro_rules! impl_callback {
    ($($first:ident$(, $($other:ident),+)?$(,)?)?) => {
        impl<$($first$(, $($other),+)?,)? Func, Output> Callback<($($first$(, $($other),+)?)?,), Output> for Func where Func: Fn($($first$(, $($other),+)?,)?) -> Output, $($first: Argable$(, $($other: Argable),+)?,)? {
            unsafe fn call(&self, args: &mut TypeMap) -> Output {
                // SAFETY: We already asserted that the callback signature is legal.
                self($($first::get(args)$(, $($other::get(args)),+)?,)?)
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

/// A `pufferfish` application.
///
/// The `App` stores the state of the application as well as any callbacks that
/// are added.
pub struct App {
    title: Cow<'static, str>,
    size: (u32, u32),
    vsync: bool,
    resizable: bool,
    state: TypeMap,
    frame_callbacks: Box<dyn Fn(&mut TypeMap)>,
    init_callbacks: Box<dyn Fn(&mut TypeMap)>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            title: "Pufferfish".into(),
            size: (800, 600),
            vsync: true,
            resizable: true,
            state: TypeMap::new(),
            frame_callbacks: Box::new(|_| {}),
            init_callbacks: Box::new(|_| {}),
        }
    }
}

impl App {
    /// Creates a new `App` with default configurations.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the title of the application window.
    ///
    /// The default value is `"Pufferfish"`.
    pub fn with_title(mut self, title: impl Into<Cow<'static, str>>) -> Self {
        self.title = title.into();
        self
    }

    /// Sets the size of the application window.
    ///
    /// The default value is `(800, 600)`.
    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Requests whether or not vsync should be enabled.
    ///
    /// The default value is `true`.
    pub fn with_vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    /// Sets whether or not the application window should be resizable.
    ///
    /// The default value is `true`.
    pub fn with_resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Adds new state to the application.
    ///
    /// If multiple values of the same type are added, the last one added will
    /// be used.
    pub fn add_state<T: 'static>(mut self, state: T) -> Self {
        self.state.insert(state);
        self
    }

    /// Adds new state to the application based on preexisting state.
    ///
    /// Closures registered via this function are executed in the order they are
    /// added when the application is initialized, alongside callbacks
    /// registered via [`add_init_callback`], and the returned value is
    /// added to the application state.
    ///
    /// [`add_init_callback`]: Self::add_init_callback
    pub fn add_state_with<T: 'static, Args, F: Callback<Args, T> + 'static>(
        mut self,
        callback: F,
    ) -> Self {
        F::assert_legal();
        replace_with(&mut self.init_callbacks, |cbs| {
            Box::new(move |args| unsafe {
                cbs(args);
                let state = callback.call(args);
                args.insert(state);
            })
        });
        self
    }

    /// Adds a callback that is executed every frame.
    ///
    /// Frame callbacks are executed in the order they are added.
    pub fn add_frame_callback<Args, F: Callback<Args, ()> + 'static>(
        mut self,
        callback: F,
    ) -> Self {
        F::assert_legal();
        replace_with(&mut self.frame_callbacks, |cbs| {
            Box::new(move |args| unsafe {
                cbs(args);
                callback.call(args);
            })
        });
        self
    }

    /// Adds a callback that is executed once when the application is
    /// initialized.
    ///
    /// Init callbacks are executed in the order they are
    /// added.
    pub fn add_init_callback<Args, F: Callback<Args, ()> + 'static>(mut self, callback: F) -> Self {
        F::assert_legal();
        replace_with(&mut self.init_callbacks, |cbs| {
            Box::new(move |args| unsafe {
                cbs(args);
                callback.call(args);
            })
        });
        self
    }

    /// Runs the application, executing any init callbacks, opening a window,
    /// and starting the event loop.
    pub fn run(self) {
        backend::run(self);
    }

    fn init(&mut self, ctx: &Rc<Context>, resource_manager: &ResourceManager) {
        self.state.insert(resource_manager.clone());
        self.state.insert(Graphics::new(ctx, resource_manager));
        self.state.insert(Input::new());

        let mut assets = Assets::new(resource_manager);

        #[cfg(feature = "png-decoder")]
        {
            let ctx = ctx.clone();
            assets.add_loader(["png"], move |bytes, _| {
                let (meta, data) = png_decoder::decode(bytes).unwrap();
                Sprite::new(
                    &ctx,
                    meta.width,
                    meta.height,
                    fugu::ImageFormat::Rgba8,
                    fugu::ImageFilter::Nearest,
                    fugu::ImageWrap::Clamp,
                    data,
                )
            });
        }

        #[cfg(feature = "text")]
        {
            use crate::text::Font;

            assets.add_loader(["ttf", "otf"], |bytes, _| Font::new(bytes));
        }

        self.state.insert(assets);

        (self.init_callbacks)(&mut self.state);
    }
}
