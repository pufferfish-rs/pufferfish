use pufferfish::assets::Assets;
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

fn main() {
    App::new()
        .with_title("Hello World")
        .with_size(500, 500)
        .add_state(State {
            t: 0.,
            player: Player { x: 250., y: 250. },
        })
        .add_frame_callback(process_input)
        .add_frame_callback(draw)
        .run();
}

fn process_input(state: &mut State, input: &Input) {
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
}

fn draw(state: &State, g: &mut Graphics, assets: &mut Assets) {
    g.clear(Color::BLACK);
    g.draw_sprite(
        state.player.x - 16.,
        state.player.y - 16.,
        assets.load("examples/player.png"),
    );
    g.draw_text(10., 10., "Hello, world!\nThis is a pufferfish example.")
        .color(Color::RED);
    g.end();
}
