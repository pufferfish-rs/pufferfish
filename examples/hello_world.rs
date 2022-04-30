use pufferfish::input::Input;
use pufferfish::App;

struct State {
    x: u64,
}

fn main() {
    App::new()
        .with_title("Hello World")
        .with_size(1024, 768)
        .add_state(State { x: 0 })
        .add_callback(|state: &mut State, input: &Input| {
            state.x += 1;
            println!(
                "[Frame {}] Keys down: {:?}",
                state.x,
                input.get_keys_down().collect::<Vec<_>>()
            );
        })
        .run();
}
