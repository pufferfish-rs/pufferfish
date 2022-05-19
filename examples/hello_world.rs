use pufferfish::assets::Assets;
use pufferfish::graphics::{Color, Graphics, Sprite};
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
        .with_size(1024, 768)
        .add_state(State {
            t: 0.,
            player: Player { x: 512., y: 384. },
        })
        .add_init_callback(init_png_loader)
        .add_frame_callback(process_input)
        .add_frame_callback(draw)
        .run();
}

fn init_png_loader(g: &Graphics, assets: &mut Assets) {
    let ctx = g.ctx.clone();
    assets.add_loader("png", move |bytes| {
        let (meta, data) = png_decoder::decode(bytes).unwrap();
        Sprite::new(
            &ctx,
            meta.width,
            meta.height,
            fugu::ImageFormat::Rgba8,
            fugu::ImageFilter::Nearest,
            fugu::ImageWrap::Clamp,
            &data,
        )
    });
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
    g.clear(Color::from_rgb(0., 0., 0.));
    g.draw_sprite(
        state.player.x - 16.,
        state.player.y - 16.,
        assets.load("examples/player.png"),
    );
    g.end();
}
