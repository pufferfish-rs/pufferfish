use pufferfish::graphics::{Color, Graphics};
use pufferfish::input::{Input, KeyCode};
use pufferfish::App;

struct Player {
    x: f32,
    y: f32,
}

struct State {
    t: f32,
    player: Player,
}

fn rainbow(t: f32) -> Color {
    let h = t.fract() * 6.;
    let (r, g, b) = match h.floor() as u16 {
        0 => (1., h.fract(), 0.),
        1 => (1. - h.fract(), 1., 0.),
        2 => (0., 1., h.fract()),
        3 => (0., 1. - h.fract(), 1.),
        4 => (h.fract(), 0., 1.),
        _ => (1., 0., 1. - h.fract()),
    };
    Color::from_rgb(r, g, b)
}

fn main() {
    App::new()
        .with_title("Hello World")
        .with_size(1024, 768)
        .add_state(State {
            t: 0.,
            player: Player { x: 512., y: 384. },
        })
        .add_callback(|state: &mut State, input: &Input| {
            state.t += 0.001;

            if input.is_key_down(KeyCode::D) {
                state.player.x += 3.;
            }
            if input.is_key_down(KeyCode::A) {
                state.player.x -= 3.;
            }
            if input.is_key_down(KeyCode::W) {
                state.player.y -= 3.;
            }
            if input.is_key_down(KeyCode::S) {
                state.player.y += 3.;
            }
        })
        .add_callback(|state: &State, g: &mut Graphics| {
            g.clear(Color::from_rgb(0., 0., 0.));
            g.set_color(rainbow(state.t));
            g.draw_rect(state.player.x - 16., state.player.y - 16., 32., 32.);
            g.end();
        })
        .run();
}
