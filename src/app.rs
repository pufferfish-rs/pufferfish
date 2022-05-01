use std::any::{type_name, Any, TypeId};
use std::borrow::Cow;
use std::cell::{Cell, UnsafeCell};
use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use fugu::Context;
use sdl2::event::Event;
use sdl2::video::GLProfile;

use crate::input::{self, Input};

#[derive(Clone, Copy)]
enum BorrowState {
    Free,
    Shared,
    Exclusive,
}

struct ManualCell<T> {
    value: UnsafeCell<T>,
    borrow: Cell<BorrowState>,
}

#[derive(Debug)]
enum ManualCellError {
    AlreadyBorrowed,
    AlreadyBorrowedMut,
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

    fn try_borrow(&self) -> Result<&T, ManualCellError> {
        match self.borrow.get() {
            BorrowState::Free | BorrowState::Shared => {
                self.borrow.set(BorrowState::Shared);
                unsafe { Ok(&*self.value.get()) }
            }
            BorrowState::Exclusive => Err(ManualCellError::AlreadyBorrowedMut),
        }
    }

    fn try_borrow_mut(&self) -> Result<&mut T, ManualCellError> {
        match self.borrow.get() {
            BorrowState::Free => {
                self.borrow.set(BorrowState::Exclusive);
                unsafe { Ok(&mut *self.value.get()) }
            }
            BorrowState::Shared => Err(ManualCellError::AlreadyBorrowed),
            BorrowState::Exclusive => Err(ManualCellError::AlreadyBorrowedMut),
        }
    }
}

struct ManualCellGuard<'a, T>(&'a mut T, &'a Cell<BorrowState>);

impl<'a, T> Drop for ManualCellGuard<'a, T> {
    fn drop(&mut self) {
        self.1.set(BorrowState::Free);
    }
}

impl<'a, T> Deref for ManualCellGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.0
    }
}

impl<'a, T> DerefMut for ManualCellGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut *self.0
    }
}

impl<T: DerefMut<Target = dyn Any>> ManualCell<T> {
    fn take_downcast_guarded<'a, V: 'static>(&'a self) -> ManualCellGuard<'a, V> {
        ManualCellGuard(
            self.try_borrow_mut()
                .expect("cannot take borrowed value")
                .downcast_mut()
                .unwrap(),
            &self.borrow,
        )
    }
}

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

trait Argable {
    unsafe fn get(args: *mut HashMap<TypeId, ManualCell<Box<dyn Any>>>) -> Self;
}

impl<T: 'static> Argable for &T {
    unsafe fn get(args: *mut HashMap<TypeId, ManualCell<Box<dyn Any>>>) -> Self {
        args.as_mut()
            .unwrap()
            .get(&TypeId::of::<T>())
            .unwrap()
            .try_borrow()
            .unwrap_or_else(|e| match e {
                ManualCellError::AlreadyBorrowed => unreachable!(),
                ManualCellError::AlreadyBorrowedMut => panic!(
                    "cannot borrow {} immutably because it was already borrowed mutably",
                    type_name::<T>().rsplit("::").next().unwrap()
                ),
            })
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
            .try_borrow_mut()
            .unwrap_or_else(|e| match e {
                ManualCellError::AlreadyBorrowed => panic!(
                    "cannot borrow {} mutably because it was already borrowed immutably",
                    type_name::<T>().rsplit("::").next().unwrap()
                ),
                ManualCellError::AlreadyBorrowedMut => panic!(
                    "cannot borrow {} mutably more than once",
                    type_name::<T>().rsplit("::").next().unwrap()
                ),
            })
            .downcast_mut()
            .unwrap()
    }
}

pub struct CallbackArgs<'a>(&'a mut HashMap<TypeId, ManualCell<Box<dyn Any>>>);

pub trait Callback<Args> {
    fn call(&self, args: CallbackArgs);
}

macro_rules! impl_callback {
    ($($first:ident$(, $($other:ident),+)?$(,)?)?) => {
        impl<$($first$(, $($other),+)?,)? Func> Callback<($($first$(, $($other),+)?)?,)> for Func where Func: Fn($($first$(, $($other),+)?,)?), $($first: Argable$(, $($other: Argable),+)?,)? {
            fn call(&self, args: CallbackArgs) {
                unsafe { self($($first::get(args.0)$(, $($other::get(args.0)),+)?,)?) }
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
                callback.call(CallbackArgs(args));
                for cell in args.values_mut() {
                    cell.free();
                }
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

        self.state.insert(
            TypeId::of::<Input>(),
            ManualCell::new(Box::new(Input::new())),
        );

        'running: loop {
            {
                let mut input = self
                    .state
                    .get(&TypeId::of::<Input>())
                    .unwrap()
                    .take_downcast_guarded::<Input>();

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
