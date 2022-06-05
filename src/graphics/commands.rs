use super::{Color, Graphics, Sprite};
use crate::assets::ResourceHandle;
use crate::text::Font;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Vertex {
    pos: (f32, f32),
    color: Color,
    uv: (f32, f32),
}

pub(super) struct DrawCommand {
    pub sprite: Option<ResourceHandle<Sprite>>,
    pub verts: Vec<Vertex>,
    pub indices: Vec<u16>,
}

/// A rectangle to be drawn.
///
/// This is a builder struct that allows you to specify extra parameters for the
/// rectangle via method chaining. The rectangle is commited to the [`Graphics`]
/// struct when [`DrawRect`] is dropped.
///
/// This struct is created using the [`draw_rect`] method on [`Graphics`].
///
/// [`draw_rect`]: Graphics::draw_rect
pub struct DrawRect<'a> {
    g: &'a mut Graphics,
    pos: (f32, f32),
    size: (f32, f32),
    color: Option<Color>,
}

impl<'a> DrawRect<'a> {
    pub(super) fn new(g: &'a mut Graphics, x: f32, y: f32, w: f32, h: f32) -> Self {
        DrawRect {
            g,
            pos: (x, y),
            size: (w, h),
            color: None,
        }
    }

    /// Sets the color of the rectangle.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    fn commit(&mut self) {
        let (x, y) = self.pos;
        let (w, h) = self.size;
        let color = self.color.unwrap_or(self.g.color);

        self.g.draw_commands.push(DrawCommand {
            sprite: None,
            verts: vec![
                Vertex {
                    pos: (x, y),
                    color,
                    uv: (0., 0.),
                },
                Vertex {
                    pos: (x + w, y),
                    color,
                    uv: (1., 0.),
                },
                Vertex {
                    pos: (x + w, y + h),
                    color,
                    uv: (1., 1.),
                },
                Vertex {
                    pos: (x, y + h),
                    color,
                    uv: (0., 1.),
                },
            ],
            indices: vec![0, 3, 1, 1, 3, 2],
        });
    }
}

impl Drop for DrawRect<'_> {
    fn drop(&mut self) {
        self.commit();
    }
}

/// A sprite to be drawn.
///
/// This is a builder struct that allows you to specify extra parameters for the
/// sprite via method chaining. The sprite is commited to the [`Graphics`]
/// struct when [`DrawSprite`] is dropped.
///
/// This struct is created using the [`draw_sprite`] method on [`Graphics`].
///
/// [`draw_sprite`]: Graphics::draw_sprite
pub struct DrawSprite<'a> {
    g: &'a mut Graphics,
    pos: (f32, f32),
    sprite: ResourceHandle<Sprite>,
    size: Option<(f32, f32)>,
    source_pos: Option<(f32, f32)>,
    source_size: Option<(f32, f32)>,
    color: Option<Color>,
}

impl<'a> DrawSprite<'a> {
    pub(super) fn new(g: &'a mut Graphics, x: f32, y: f32, sprite: ResourceHandle<Sprite>) -> Self {
        DrawSprite {
            g,
            sprite,
            pos: (x, y),
            size: None,
            source_pos: None,
            source_size: None,
            color: None,
        }
    }

    /// Sets the size of the sprite.
    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = Some((w, h));
        self
    }

    /// Sets the source position of the sprite.
    pub fn source_pos(mut self, x: f32, y: f32) -> Self {
        self.source_pos = Some((x, y));
        self
    }

    /// Sets the source size of the sprite.
    pub fn source_size(mut self, w: f32, h: f32) -> Self {
        self.source_size = Some((w, h));
        self
    }

    /// Sets the color of the sprite.
    pub fn source_rect(mut self, x: f32, y: f32, w: f32, h: f32) -> Self {
        self.source_pos = Some((x, y));
        self.source_size = Some((w, h));
        self
    }

    fn commit(&mut self) -> Option<()> {
        let sprite = self.g.resource_manager.get(self.sprite)?;
        let w = sprite.width as f32;
        let h = sprite.height as f32;

        let (dx, dy) = self.pos;
        let (dw, dh) = self.size.or(self.source_size).unwrap_or((w, h));
        let (sx, sy) = self
            .source_pos
            .map(|(sx, sy)| (sx / w, sy / h))
            .unwrap_or((0., 0.));
        let (sw, sh) = self
            .source_size
            .map(|(sw, sh)| (sw / w, sh / h))
            .unwrap_or((1., 1.));
        let color = self.color.unwrap_or(self.g.color);

        self.g.draw_commands.push(DrawCommand {
            sprite: Some(self.sprite),
            verts: vec![
                Vertex {
                    pos: (dx, dy),
                    color,
                    uv: (sx, sy),
                },
                Vertex {
                    pos: (dx + dw, dy),
                    color,
                    uv: (sx + sw, sy),
                },
                Vertex {
                    pos: (dx + dw, dy + dh),
                    color,
                    uv: (sx + sw, sy + sh),
                },
                Vertex {
                    pos: (dx, dy + dh),
                    color,
                    uv: (sx, sy + sh),
                },
            ],
            indices: vec![0, 3, 1, 1, 3, 2],
        });

        Some(())
    }
}

impl Drop for DrawSprite<'_> {
    fn drop(&mut self) {
        self.commit();
    }
}

/// Text to be drawn.
///
/// This is a builder struct that allows you to specify extra parameters for the
/// text via method chaining. The text is commited to the [`Graphics`]
/// struct when [`DrawText`] is dropped.
///
/// This struct is created using the [`draw_text`] method on [`Graphics`].
///
/// [`draw_text`]: Graphics::draw_text
#[cfg(feature = "text")]
pub struct DrawText<'a> {
    g: &'a mut Graphics,
    pos: (f32, f32),
    text: &'a str,
    font: Option<ResourceHandle<Font>>,
    size: Option<f32>,
    color: Option<Color>,
}

#[cfg(feature = "text")]
impl<'a> DrawText<'a> {
    pub(super) fn new(g: &'a mut Graphics, x: f32, y: f32, text: &'a str) -> Self {
        DrawText {
            g,
            pos: (x, y),
            text,
            font: None,
            size: None,
            color: None,
        }
    }

    /// Sets the font of the text.
    pub fn font(mut self, font: ResourceHandle<Font>) -> Self {
        self.font = Some(font);
        self
    }

    /// Sets the size of the text.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    /// Sets the color of the text.
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    fn commit(&mut self) {
        let (x, y) = self.pos;
        let text = self.text;
        let font = self.font.unwrap_or_else(|| self.g.default_font());
        let size = self.size.unwrap_or(32.);
        crate::text::draw_text(self.g, x, y, text, font, size);
    }
}

#[cfg(feature = "text")]
impl Drop for DrawText<'_> {
    fn drop(&mut self) {
        self.commit();
    }
}
