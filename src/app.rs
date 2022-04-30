use std::any::{Any, TypeId};
use std::borrow::Cow;
use std::cell::{Cell, UnsafeCell};
use std::collections::HashMap;

use fugu::Context;
use sdl2::event::Event;
use sdl2::video::GLProfile;

#[derive(Clone, Copy)]
enum BorrowState {
    Free,
    Shared,
    Exclusive,
}

pub struct ManualCell<T> {
    value: UnsafeCell<T>,
    borrow: Cell<BorrowState>,
}

impl<T> ManualCell<T> {
    fn new(value: T) -> ManualCell<T> {
        ManualCell {
            value: UnsafeCell::new(value),
            borrow: Cell::new(BorrowState::Free),
        }
    }

    fn free(&self) {
        self.borrow.set(BorrowState::Free);
    }

    fn borrow(&self) -> &T {
        match self.borrow.get() {
            BorrowState::Free | BorrowState::Shared => {
                self.borrow.set(BorrowState::Shared);
                unsafe { &*self.value.get() }
            }
            BorrowState::Exclusive => panic!("already mutably borrowed"),
        }
    }

    fn borrow_mut(&self) -> &mut T {
        match self.borrow.get() {
            BorrowState::Free => {
                self.borrow.set(BorrowState::Exclusive);
                unsafe { &mut *self.value.get() }
            }
            BorrowState::Shared => panic!("already borrowed"),
            BorrowState::Exclusive => panic!("already mutably borrowed"),
        }
    }
}

fn replace_with<T, F: FnOnce(T) -> T>(dest: &mut T, f: F) {
    unsafe {
        let old = std::ptr::read(dest);
        let new = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(old)))
            .unwrap_or_else(|_| std::process::abort());
        std::ptr::write(dest, new);
    }
}

trait Argable {
    unsafe fn get(args: *mut HashMap<TypeId, ManualCell<Box<dyn Any>>>) -> Self;
}

impl<T: 'static> Argable for &T {
    unsafe fn get(args: *mut HashMap<TypeId, ManualCell<Box<dyn Any>>>) -> Self {
        args.as_mut()
            .unwrap()
            .get(&TypeId::of::<T>())
            .unwrap()
            .borrow()
            .downcast_ref()
            .unwrap()
    }
}

impl<T: 'static> Argable for &mut T {
    unsafe fn get(args: *mut HashMap<TypeId, ManualCell<Box<dyn Any>>>) -> Self {
        args.as_mut()
            .unwrap()
            .get(&TypeId::of::<T>())
            .unwrap()
            .borrow_mut()
            .downcast_mut()
            .unwrap()
    }
}

pub trait Callback<Args> {
    fn call(&self, args: &mut HashMap<TypeId, ManualCell<Box<dyn Any>>>);
}

macro_rules! impl_callback {
    ($($first:ident$(, $($other:ident),+)?$(,)?)?) => {
        impl<$($first$(, $($other),+)?,)? Func> Callback<($($first$(, $($other),+)?)?,)> for Func where Func: Fn($($first$(, $($other),+)?,)?), $($first: Argable$(, $($other: Argable),+)?,)? {
            fn call(&self, args: &mut HashMap<TypeId, ManualCell<Box<dyn Any>>>) {
                unsafe { self($($first::get(args)$(, $($other::get(args)),+)?,)?) }
            }
        }
        $($(impl_callback!($($other),+);)?)?
    };
}

impl_callback!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

pub struct App {
    title: Cow<'static, str>,
    size: (u32, u32),
    state: HashMap<TypeId, ManualCell<Box<dyn Any>>>,
    callbacks: Box<dyn Fn(&mut HashMap<TypeId, ManualCell<Box<dyn Any>>>)>,
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
            .insert(TypeId::of::<T>(), ManualCell::new(Box::new(state)));
        self
    }

    pub fn add_callback<'a, Args, T: Callback<Args> + 'static>(mut self, callback: T) -> App {
        replace_with(&mut self.callbacks, |cbs| {
            Box::new(move |args: &mut _| {
                cbs(args);
                callback.call(args);
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
        let _ctx = Context::new(|s| video_subsystem.gl_get_proc_address(s).cast());

        let mut event_pump = sdl_context.event_pump().unwrap();

        'running: loop {
            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'running,
                    _ => {}
                }
            }

            (self.callbacks.as_ref())(&mut self.state);

            for cell in self.state.values_mut() {
                cell.free();
            }

            window.gl_swap_window();
        }
    }
}
