//! An opinionated 2D game library for Rust.
//!
//! [![Docs](https://docs.rs/pufferfish/badge.svg)](https://docs.rs/pufferfish)
//! [![Crates.io](https://shields.io/crates/v/pufferfish)](https://crates.io/crates/pufferfish)
//! ![License](https://shields.io/crates/l/pufferfish)
//!
//! ### Features
//!
//! - Minimal and opinionated API
//! - Simple but flexible callback system
//! - Easy input handling via polling
//! - Efficient 2D renderer with sprite batching, powered by [`fugu`](https://github.com/pufferfish-rs/fugu)
//! - Asset loader with support for custom formats
//!
//! ### Getting Started
//!
//! To add pufferfish to your project, add the following to the dependencies
//! section of your `Cargo.toml`:
//!
//! ```toml
//! pufferfish = "0.1"
//! ```
//!
//! See the `examples/` directory in the source to get a feel of how
//! pufferfish's API works.
//!
//! A basic pufferfish program looks something like this:
//!
//! ```rust
//! use pufferfish::graphics::{Color, Graphics};
//! use pufferfish::App;
//!
//! struct State {
//!     // Your game state...
//! }
//!
//! fn main() {
//!     App::new()
//!         .with_title("Hello World")
//!         .add_state(State::new()) // Add your state
//!         .add_init_callback(init) // Add your callbacks
//!         .add_frame_callback(update)
//!         .add_frame_callback(draw)
//!         .run();
//! }
//!
//! fn init(state: &mut State) {
//!     // Initialization code here...
//! }
//!
//! fn update(state: &mut State) {
//!     // Update code here...
//! }
//!
//! // Request arbitrary state through the callback's type signature
//! fn draw(state: &State, g: &Graphics) {
//!     g.clear(Color::from_rgb(0., 0., 0.));
//!     g.begin();
//!     // Draw code here...
//!     g.end();
//! }
//! ```

#![warn(missing_docs)]

mod app;
pub use app::*;

pub mod assets;
pub mod graphics;
pub mod input;
pub mod text;

mod util;
