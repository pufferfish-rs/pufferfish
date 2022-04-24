use pufferfish::App;

fn main() {
    App::new()
        .with_title("Hello World")
        .with_size(1024, 768)
        .run();
}
