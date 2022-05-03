use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::UnsafeCell;
use std::collections::HashMap;

use fugu::Context;
use sdl2::event::Event;
use sdl2::video::GLProfile;

use crate::graphics::Graphics;
use crate::input::{self, Input};

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
    unsafe fn get(args: *mut HashMap<TypeId, UnsafeCell<Box<dyn Any>>>) -> Self;
    fn desc() -> ArgDesc;
}

impl<T: 'static> Argable for &T {
    unsafe fn get(args: *mut HashMap<TypeId, UnsafeCell<Box<dyn Any>>>) -> Self {
        args.as_mut()
            .unwrap()
            .get(&TypeId::of::<T>())
            .unwrap()
            .get()
            .as_ref()
            .unwrap_unchecked()
            .downcast_ref()
            .unwrap_unchecked()
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
    unsafe fn get(args: *mut HashMap<TypeId, UnsafeCell<Box<dyn Any>>>) -> Self {
        args.as_mut()
            .unwrap()
            .get(&TypeId::of::<T>())
            .unwrap()
            .get()
            .as_mut()
            .unwrap_unchecked()
            .downcast_mut()
            .unwrap_unchecked()
    }

    fn desc() -> ArgDesc {
        ArgDesc {
            tid: TypeId::of::<T>(),
            tname: type_name::<T>(),
            unique: true,
        }
    }
}

pub struct CallbackArgs<'a>(&'a mut HashMap<TypeId, UnsafeCell<Box<dyn Any>>>);

pub trait Callback<Args> {
    fn call(&self, args: CallbackArgs);
    fn assert_legal();
}

macro_rules! impl_callback {
    ($($first:ident$(, $($other:ident),+)?$(,)?)?) => {
        impl<$($first$(, $($other),+)?,)? Func> Callback<($($first$(, $($other),+)?)?,)> for Func where Func: Fn($($first$(, $($other),+)?,)?), $($first: Argable$(, $($other: Argable),+)?,)? {
            fn call(&self, args: CallbackArgs) {
                unsafe { self($($first::get(args.0)$(, $($other::get(args.0)),+)?,)?) }
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
    state: HashMap<TypeId, UnsafeCell<Box<dyn Any>>>,
    callbacks: Box<dyn Fn(&mut HashMap<TypeId, UnsafeCell<Box<dyn Any>>>)>,
}

impl App {
    pub fn new() -> App {
        App {
            title: "Pufferfish".into(),
            size: (800, 600),
            state: HashMap::new(),
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

    pub fn add_state<T: 'static>(mut self, state: T) -> App {
        self.state
            .insert(TypeId::of::<T>(), UnsafeCell::new(Box::new(state)));
        self
    }

    pub fn add_callback<'a, Args, T: Callback<Args> + 'static>(mut self, callback: T) -> App {
        T::assert_legal();
        replace_with(&mut self.callbacks, |cbs| {
            Box::new(move |args: &mut _| {
                cbs(args);
                callback.call(CallbackArgs(args));
            })
        });
        self
    }

    pub fn run(mut self) {
        let sdl_context = sdl2::init().unwrap();
        let video_subsystem = sdl_context.video().unwrap();

        let window = video_subsystem
            .window(&self.title, self.size.0, self.size.1)
            .position_centered()
            .opengl()
            .build()
            .unwrap();

        let gl_attr = video_subsystem.gl_attr();
        gl_attr.set_context_version(3, 3);
        gl_attr.set_context_profile(GLProfile::Core);

        let _gl = window.gl_create_context().unwrap();
        let ctx = Context::new(|s| video_subsystem.gl_get_proc_address(s).cast());

        let mut event_pump = sdl_context.event_pump().unwrap();

        let mut graphics = Graphics::new(ctx);
        graphics.set_viewport(self.size);
        self.state.insert(
            TypeId::of::<Graphics>(),
            UnsafeCell::new(Box::new(graphics)),
        );

        self.state.insert(
            TypeId::of::<Input>(),
            UnsafeCell::new(Box::new(Input::new())),
        );

        'running: loop {
            {
                let input = unsafe {
                    (&mut *self.state.get(&TypeId::of::<Input>()).unwrap().get())
                        .downcast_mut::<Input>()
                        .unwrap_unchecked()
                };

                input.update();

                for event in event_pump.poll_iter() {
                    match event {
                        Event::Quit { .. } => break 'running,
                        Event::KeyDown {
                            keycode, repeat, ..
                        } => {
                            if let (Some(key), false) = (input::keycode_from_sdl(keycode), repeat) {
                                if !input.keys_down.contains(&key) {
                                    input.keys_down.push(key);
                                }
                                if !input.keys_pressed.contains(&key) {
                                    input.keys_pressed.push(key);
                                }
                            }
                        }
                        Event::KeyUp {
                            keycode, repeat, ..
                        } => {
                            if let (Some(key), false) = (input::keycode_from_sdl(keycode), repeat) {
                                input.keys_down.retain(|&k| k != key);
                                input.keys_released.push(key);
                            }
                        }
                        _ => {}
                    }
                }
            }

            (self.callbacks.as_ref())(&mut self.state);

            window.gl_swap_window();
        }
    }
}
