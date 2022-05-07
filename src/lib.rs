#[cfg(any(
    not(any(feature = "sdl", feature = "glutin")),
    all(feature = "sdl", feature = "glutin")
))]
compile_error!("exactly one of features `sdl2` and `glutin` must be enabled");

mod app;
pub use app::*;

pub mod graphics;
pub mod input;
