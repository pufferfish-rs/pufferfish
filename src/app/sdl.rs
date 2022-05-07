#![cfg(feature = "sdl")]

use fugu::Context;
use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Keycode as SDLKeyCode;
use sdl2::video::GLProfile;

use crate::graphics::Graphics;
use crate::input::{Input, KeyCode};
use crate::App;

pub fn run(mut app: App) {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut window_builder = video_subsystem.window(&app.title, app.size.0, app.size.1);

    window_builder.opengl();

    if app.resizable {
        window_builder.resizable();
    }

    let window = window_builder.build().unwrap();

    video_subsystem
        .gl_set_swap_interval(if app.vsync { 1 } else { 0 })
        .ok();
    let gl_attr = video_subsystem.gl_attr();
    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_profile(GLProfile::Core);

    let _gl = window.gl_create_context().unwrap();
    let ctx = Context::new(|s| video_subsystem.gl_get_proc_address(s).cast());

    let mut event_pump = sdl_context.event_pump().unwrap();

    app.init(ctx);

    {
        let graphics = unsafe { app.state.get_mut::<Graphics>().unwrap_unchecked() };
        graphics.set_viewport((app.size.0, app.size.1));
    }

    'running: loop {
        {
            let input = unsafe { app.state.get_mut::<Input>().unwrap_unchecked() };
            input.update();

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit { .. } => break 'running,
                    Event::Window { win_event, .. } => match win_event {
                        WindowEvent::Resized(w, h) => {
                            let graphics =
                                unsafe { app.state.get_mut::<Graphics>().unwrap_unchecked() };
                            graphics.set_viewport((w as u32, h as u32));
                        }
                        _ => {}
                    },
                    Event::KeyDown {
                        keycode, repeat, ..
                    } => {
                        if let (Some(key), false) = (convert_keycode(keycode), repeat) {
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
                        if let (Some(key), false) = (convert_keycode(keycode), repeat) {
                            input.keys_down.retain(|&k| k != key);
                            input.keys_released.push(key);
                        }
                    }
                    _ => {}
                }
            }
        }

        (app.callbacks.as_ref())(&mut app.state);

        window.gl_swap_window();
    }
}

fn convert_keycode(keycode: Option<SDLKeyCode>) -> Option<KeyCode> {
    keycode.and_then(|keycode| match keycode {
        SDLKeyCode::A => Some(KeyCode::A),
        SDLKeyCode::B => Some(KeyCode::B),
        SDLKeyCode::C => Some(KeyCode::C),
        SDLKeyCode::D => Some(KeyCode::D),
        SDLKeyCode::E => Some(KeyCode::E),
        SDLKeyCode::F => Some(KeyCode::F),
        SDLKeyCode::G => Some(KeyCode::G),
        SDLKeyCode::H => Some(KeyCode::H),
        SDLKeyCode::I => Some(KeyCode::I),
        SDLKeyCode::J => Some(KeyCode::J),
        SDLKeyCode::K => Some(KeyCode::K),
        SDLKeyCode::L => Some(KeyCode::L),
        SDLKeyCode::M => Some(KeyCode::M),
        SDLKeyCode::N => Some(KeyCode::N),
        SDLKeyCode::O => Some(KeyCode::O),
        SDLKeyCode::P => Some(KeyCode::P),
        SDLKeyCode::Q => Some(KeyCode::Q),
        SDLKeyCode::R => Some(KeyCode::R),
        SDLKeyCode::S => Some(KeyCode::S),
        SDLKeyCode::T => Some(KeyCode::T),
        SDLKeyCode::U => Some(KeyCode::U),
        SDLKeyCode::V => Some(KeyCode::V),
        SDLKeyCode::W => Some(KeyCode::W),
        SDLKeyCode::X => Some(KeyCode::X),
        SDLKeyCode::Y => Some(KeyCode::Y),
        SDLKeyCode::Z => Some(KeyCode::Z),

        SDLKeyCode::Num0 => Some(KeyCode::Alpha0),
        SDLKeyCode::Num1 => Some(KeyCode::Alpha1),
        SDLKeyCode::Num2 => Some(KeyCode::Alpha2),
        SDLKeyCode::Num3 => Some(KeyCode::Alpha3),
        SDLKeyCode::Num4 => Some(KeyCode::Alpha4),
        SDLKeyCode::Num5 => Some(KeyCode::Alpha5),
        SDLKeyCode::Num6 => Some(KeyCode::Alpha6),
        SDLKeyCode::Num7 => Some(KeyCode::Alpha7),
        SDLKeyCode::Num8 => Some(KeyCode::Alpha8),
        SDLKeyCode::Num9 => Some(KeyCode::Alpha9),

        SDLKeyCode::LCtrl => Some(KeyCode::LeftControl),
        SDLKeyCode::LShift => Some(KeyCode::LeftShift),
        SDLKeyCode::LAlt => Some(KeyCode::LeftAlt),
        SDLKeyCode::RCtrl => Some(KeyCode::RightControl),
        SDLKeyCode::RShift => Some(KeyCode::RightShift),
        SDLKeyCode::RAlt => Some(KeyCode::RightAlt),

        _ => None,
    })
}
