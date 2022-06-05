/// A linear RGBA color represented by 4 [f32]s.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color {
    /// The red component of the color.
    pub r: f32,
    /// The green component of the color.
    pub g: f32,
    /// The blue component of the color.
    pub b: f32,
    /// The alpha component of the color.
    pub a: f32,
}

impl Color {
    /// Creates a new color with the given components.
    pub const fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    /// Creates a new color with the given components and an alpha of 1.
    pub const fn from_rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1. }
    }
}

#[allow(missing_docs)]
impl Color {
    pub const BLACK: Color = Color::from_rgb(0., 0., 0.);
    pub const WHITE: Color = Color::from_rgb(1., 1., 1.);
    pub const RED: Color = Color::from_rgb(1., 0., 0.);
    pub const GREEN: Color = Color::from_rgb(0., 1., 0.);
    pub const BLUE: Color = Color::from_rgb(0., 0., 1.);
    pub const YELLOW: Color = Color::from_rgb(1., 1., 0.);
    pub const CYAN: Color = Color::from_rgb(0., 1., 1.);
    pub const MAGENTA: Color = Color::from_rgb(1., 0., 1.);
    pub const TRANSPARENT: Color = Color::from_rgba(0., 0., 0., 0.);
}
