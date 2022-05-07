#![cfg(feature = "glutin")]

use fugu::Context;
use glutin::dpi::PhysicalSize;
use glutin::event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use glutin::event_loop::{ControlFlow, EventLoop};
use glutin::window::WindowBuilder;
use glutin::ContextBuilder;

use crate::graphics::Graphics;
use crate::input::{Input, KeyCode};
use crate::App;

pub fn run(mut app: App) {
    let el = EventLoop::new();
    let wb = WindowBuilder::new()
        .with_title(app.title.to_string())
        .with_inner_size(PhysicalSize::new(app.size.0, app.size.1))
        .with_resizable(app.resizable);

    let windowed_context = ContextBuilder::new()
        .with_vsync(app.vsync)
        .build_windowed(wb, &el)
        .unwrap();

    let windowed_context = unsafe { windowed_context.make_current().unwrap() };

    let ctx = Context::new(|s| windowed_context.context().get_proc_address(s));
    ctx.set_viewport(0, 0, app.size.0, app.size.1);

    app.init(ctx);

    {
        let graphics = unsafe { app.state.get_mut::<Graphics>().unwrap_unchecked() };
        graphics.set_viewport((app.size.0, app.size.1));
    }

    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        let input = unsafe { app.state.get_mut::<Input>().unwrap_unchecked() };
        input.update();

        match event {
            Event::LoopDestroyed => return,
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::Resized(size) => {
                    let graphics = unsafe { app.state.get_mut::<Graphics>().unwrap_unchecked() };
                    graphics.set_viewport((size.width, size.height));
                }
                WindowEvent::KeyboardInput {
                    input:
                        KeyboardInput {
                            virtual_keycode,
                            state,
                            ..
                        },
                    ..
                } => match state {
                    ElementState::Pressed => {
                        if let Some(key) = convert_keycode(virtual_keycode) {
                            if !input.keys_down.contains(&key) {
                                input.keys_down.push(key);
                                input.keys_pressed.push(key);
                            }
                        }
                    }

                    ElementState::Released => {
                        if let Some(key) = convert_keycode(virtual_keycode) {
                            input.keys_down.retain(|&k| k != key);
                            input.keys_released.push(key);
                        }
                    }
                },
                _ => (),
            },
            Event::RedrawRequested(_) => {
                drop(input);
                (app.callbacks.as_ref())(&mut app.state);
                windowed_context.swap_buffers().unwrap();
            }
            Event::MainEventsCleared => {
                windowed_context.window().request_redraw();
            }
            _ => (),
        }
    });
}

fn convert_keycode(keycode: Option<VirtualKeyCode>) -> Option<KeyCode> {
    keycode.and_then(|keycode| match keycode {
        VirtualKeyCode::A => Some(KeyCode::A),
        VirtualKeyCode::B => Some(KeyCode::B),
        VirtualKeyCode::C => Some(KeyCode::C),
        VirtualKeyCode::D => Some(KeyCode::D),
        VirtualKeyCode::E => Some(KeyCode::E),
        VirtualKeyCode::F => Some(KeyCode::F),
        VirtualKeyCode::G => Some(KeyCode::G),
        VirtualKeyCode::H => Some(KeyCode::H),
        VirtualKeyCode::I => Some(KeyCode::I),
        VirtualKeyCode::J => Some(KeyCode::J),
        VirtualKeyCode::K => Some(KeyCode::K),
        VirtualKeyCode::L => Some(KeyCode::L),
        VirtualKeyCode::M => Some(KeyCode::M),
        VirtualKeyCode::N => Some(KeyCode::N),
        VirtualKeyCode::O => Some(KeyCode::O),
        VirtualKeyCode::P => Some(KeyCode::P),
        VirtualKeyCode::Q => Some(KeyCode::Q),
        VirtualKeyCode::R => Some(KeyCode::R),
        VirtualKeyCode::S => Some(KeyCode::S),
        VirtualKeyCode::T => Some(KeyCode::T),
        VirtualKeyCode::U => Some(KeyCode::U),
        VirtualKeyCode::V => Some(KeyCode::V),
        VirtualKeyCode::W => Some(KeyCode::W),
        VirtualKeyCode::X => Some(KeyCode::X),
        VirtualKeyCode::Y => Some(KeyCode::Y),
        VirtualKeyCode::Z => Some(KeyCode::Z),

        VirtualKeyCode::Key0 => Some(KeyCode::Alpha0),
        VirtualKeyCode::Key1 => Some(KeyCode::Alpha1),
        VirtualKeyCode::Key2 => Some(KeyCode::Alpha2),
        VirtualKeyCode::Key3 => Some(KeyCode::Alpha3),
        VirtualKeyCode::Key4 => Some(KeyCode::Alpha4),
        VirtualKeyCode::Key5 => Some(KeyCode::Alpha5),
        VirtualKeyCode::Key6 => Some(KeyCode::Alpha6),
        VirtualKeyCode::Key7 => Some(KeyCode::Alpha7),
        VirtualKeyCode::Key8 => Some(KeyCode::Alpha8),
        VirtualKeyCode::Key9 => Some(KeyCode::Alpha9),

        VirtualKeyCode::LControl => Some(KeyCode::LeftControl),
        VirtualKeyCode::LShift => Some(KeyCode::LeftShift),
        VirtualKeyCode::LAlt => Some(KeyCode::LeftAlt),
        VirtualKeyCode::RControl => Some(KeyCode::RightControl),
        VirtualKeyCode::RShift => Some(KeyCode::RightShift),
        VirtualKeyCode::RAlt => Some(KeyCode::RightAlt),

        _ => None,
    })
}
