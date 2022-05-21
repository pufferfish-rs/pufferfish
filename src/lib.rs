//! An opinionated 2D game library for Rust.

#![warn(missing_docs)]

mod app;
pub use app::*;

pub mod assets;
pub mod graphics;
pub mod input;

mod util;
