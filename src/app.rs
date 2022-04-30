use std::any::{TypeId, Any};
use std::borrow::Cow;
use std::collections::HashMap;

use fugu::Context;
use sdl2::event::Event;
use sdl2::video::GLProfile;

pub trait Callback<Args> {
    fn call(&self, args: &mut HashMap<TypeId, Box<dyn Any>>);
}

impl<A: 'static, F> Callback<(A,)> for F where F: Fn(&mut A) {
    fn call(&self, args: &mut HashMap<TypeId, Box<dyn Any>>) {
        let a = args.get_mut(&TypeId::of::<A>()).unwrap().downcast_mut().unwrap();
        self(a)
    }
}

pub struct App {
    title: Cow<'static, str>,
    size: (u32, u32),
    state: HashMap<TypeId, Box<dyn Any>>,
    callbacks: Option<Box<dyn Fn(&mut HashMap<TypeId, Box<dyn Any>>)>>,
}

impl App {
    pub fn new() -> App {
        App {
            title: "Pufferfish".into(),
            size: (800, 600),
            state: HashMap::new(),
            callbacks: Some(Box::new(|_| {})),
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
        self.state.insert(TypeId::of::<T>(), Box::new(state));
        self
    }

    pub fn add_callback<Args, T: Callback<Args> + 'static>(mut self, callback: T) -> App {
        let cbs = self.callbacks.take().unwrap();
        self.callbacks = Some(Box::new(move |args: &mut _| {
            cbs(args);
            callback.call(args);
        }));
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

            (self.callbacks.as_ref().unwrap())(&mut self.state);

            window.gl_swap_window();
        }
    }
}
