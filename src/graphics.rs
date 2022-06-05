//! Types relating to graphics and drawing.

use std::rc::Rc;

use fugu::{
    BlendFactor, BlendOp, BlendState, Buffer, BufferKind, BufferLayout, BufferUsage, Context,
    Image, ImageFilter, ImageFormat, ImageUniform, ImageWrap, PassAction, Pipeline, Uniform,
    UniformFormat, VertexAttribute, VertexFormat,
};

use crate::assets::{ResourceHandle, ResourceManager};
#[cfg(feature = "text")]
use crate::text::Font;

mod color;
pub use color::Color;
pub mod commands;
use commands::*;

mod shader {
    pub const VERT: &str = r"
        #version 330
        
        uniform vec2 viewport_size;
        
        in vec2 pos;
        in vec4 color;
        in vec2 uv;
        
        out vec4 vert_color;
        out vec2 vert_uv;
        
        void main() {
            vec2 npos = pos * vec2(2, -2) / viewport_size + vec2(-1, 1);
            gl_Position = vec4(npos, 0, 1);
            vert_color = color;
            vert_uv = uv;
        }
    ";

    pub const FRAG: &str = r"
        #version 330
        
        uniform sampler2D tex;

        in vec4 vert_color;
        in vec2 vert_uv;
        
        out vec4 out_color;
        
        void main() {
            out_color = vert_color * texture(tex, vert_uv);
        }
    ";
}

/// A sprite.
pub struct Sprite {
    image: Image,
    width: u32,
    height: u32,
}

impl Sprite {
    /// Creates a new sprite from the given parameters.
    ///
    /// # Arguments
    ///
    /// * `ctx` - A reference to the [`Context`] to use to create the image.
    /// * `width` - A `u32` representing the width of the sprite.
    /// * `height` - A `u32` representing the height of the sprite.
    /// * `format` - The color format of the image.
    /// * `filter` - The filter to use when sampling the image.
    /// * `wrap` - The wrap mode to use when sampling the image.
    /// * `data` - A slice of the image data.
    pub fn new(
        ctx: &Context,
        width: u32,
        height: u32,
        format: ImageFormat,
        filter: ImageFilter,
        wrap: ImageWrap,
        data: impl AsRef<[u8]>,
    ) -> Self {
        let image = ctx.create_image_with_data(width, height, format, filter, wrap, data.as_ref());
        Self {
            image,
            width,
            height,
        }
    }

    /// Returns the sprite's underlying [`Image`].
    pub fn inner(&self) -> &Image {
        &self.image
    }

    /// Returns the width of the sprite.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the sprite.
    pub fn height(&self) -> u32 {
        self.height
    }
}

#[derive(Debug)]
struct DrawBatch {
    sprite: Option<ResourceHandle<Sprite>>,
    start: usize,
    count: usize,
}

/// An interface for hardware-accelerated 2D drawing. Accessible from
/// [`App`](crate::App) by default.
pub struct Graphics {
    /// An [`Rc`] of the underlying [`Context`].
    pub ctx: Rc<Context>,
    pub(crate) resource_manager: ResourceManager,
    pipeline: Pipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    blank_image: Image,
    default_font: Option<ResourceHandle<Font>>,
    draw_commands: Vec<DrawCommand>,
    viewport: (f32, f32),
    color: Color,
    depth: f32,
}

#[allow(clippy::too_many_arguments)]
impl Graphics {
    pub(crate) fn new(ctx: &Rc<Context>, resource_manager: &ResourceManager) -> Graphics {
        ctx.set_blend(BlendState {
            op: BlendOp::Add,
            source: BlendFactor::SourceAlpha,
            dest: BlendFactor::OneMinusSourceAlpha,
        });

        let default_shader = ctx.create_shader(
            shader::VERT,
            shader::FRAG,
            &[Uniform {
                name: "viewport_size",
                format: UniformFormat::Float2,
            }],
            &[ImageUniform { name: "tex" }],
        );
        let pipeline = ctx.create_pipeline(
            default_shader,
            &[BufferLayout::default()],
            &[
                VertexAttribute {
                    name: "pos",
                    format: VertexFormat::Float2,
                    buffer_index: 0,
                },
                VertexAttribute {
                    name: "color",
                    format: VertexFormat::Float4,
                    buffer_index: 0,
                },
                VertexAttribute {
                    name: "uv",
                    format: VertexFormat::Float2,
                    buffer_index: 0,
                },
            ],
        );
        let vertex_buffer = ctx.create_buffer(BufferKind::Vertex, BufferUsage::Stream, 10000);
        let index_buffer = ctx.create_buffer(BufferKind::Index, BufferUsage::Stream, 15000);
        let blank_image = ctx.create_image_with_data(
            1,
            1,
            ImageFormat::Rgb8,
            ImageFilter::Nearest,
            ImageWrap::Clamp,
            &[255_u8; 3],
        );
        let default_font = None;
        let draw_commands = Vec::new();
        let viewport = (0., 0.);
        let color = Color::WHITE;
        let depth = 0.;

        Graphics {
            ctx: ctx.clone(),
            resource_manager: resource_manager.clone(),
            pipeline,
            vertex_buffer,
            index_buffer,
            blank_image,
            default_font,
            draw_commands,
            viewport,
            color,
            depth,
        }
    }

    pub(crate) fn set_viewport(&mut self, (width, height): (u32, u32)) {
        self.ctx.set_viewport(0, 0, width, height);
        self.viewport = (width as f32, height as f32);
    }

    /// Immediately clears the screen to the given color.
    pub fn clear(&self, color: Color) {
        self.ctx.begin_default_pass(PassAction::Clear {
            color: Some((color.r, color.g, color.b, color.a)),
            depth: None,
            stencil: None,
        });
        self.ctx.end_render_pass();
    }

    /// Begins drawing.
    pub fn begin(&mut self) {}

    /// Sets the default color to use when drawing.
    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    /// Sets the default depth to use when drawing.
    pub fn set_depth(&mut self, depth: f32) {
        self.depth = depth;
    }

    /// Draws a rectangle at the given position with the given dimensions.
    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32) -> DrawRect {
        DrawRect::new(self, x, y, w, h)
    }

    /// Draws a sprite at the given position.
    pub fn draw_sprite(&mut self, x: f32, y: f32, sprite: ResourceHandle<Sprite>) -> DrawSprite {
        DrawSprite::new(self, x, y, sprite)
    }

    /// Draws the given text at the given position.
    #[cfg(feature = "text")]
    pub fn draw_text<'a>(&'a mut self, x: f32, y: f32, text: &'a str) -> DrawText {
        DrawText::new(self, x, y, text)
    }

    /// Gets the default font, loading it if it hasn't been loaded already.
    #[cfg(feature = "text")]
    pub fn default_font(&mut self) -> ResourceHandle<Font> {
        *self.default_font.get_or_insert_with(|| {
            let font = self.resource_manager.allocate();
            self.resource_manager
                .set(font, Font::new(include_bytes!("graphics/monogram.otf")));
            font
        })
    }

    /// Overrides the default font returned by [`default_font`].
    ///
    /// [`default_font`]: Graphics::default_font
    #[cfg(feature = "text")]
    pub fn set_default_font(&mut self, font: ResourceHandle<Font>) {
        self.default_font = Some(font);
    }

    /// Ends drawing and commits everything to the screen.
    pub fn end(&mut self) {
        if self.draw_commands.is_empty() {
            return;
        }

        self.ctx.begin_default_pass(PassAction::Nothing);

        self.ctx.set_pipeline(&self.pipeline);
        self.ctx.set_vertex_buffer(&self.vertex_buffer);
        self.ctx.set_index_buffer(&self.index_buffer);
        self.ctx.set_uniforms(self.viewport);
        self.ctx.set_images(&[&self.blank_image]);

        self.draw_commands.sort_by(|a, b| {
            f32::partial_cmp(&b.depth, &a.depth).unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut batches = Vec::new();
        let mut curr_sprite = self.draw_commands[0].sprite;
        let mut begin = 0;

        let mut verts = Vec::new();
        let mut indices = Vec::new();

        for draw_command in self.draw_commands.drain(..) {
            if curr_sprite != draw_command.sprite {
                batches.push(DrawBatch {
                    sprite: curr_sprite,
                    start: begin,
                    count: indices.len() - begin,
                });
                curr_sprite = draw_command.sprite;
                begin = indices.len();
            }
            indices.extend(
                draw_command
                    .indices
                    .iter()
                    .copied()
                    .map(|e| e + verts.len() as u16),
            );
            verts.extend_from_slice(&draw_command.verts);
        }

        batches.push(DrawBatch {
            sprite: curr_sprite,
            start: begin,
            count: indices.len() - begin,
        });

        self.vertex_buffer.update(&verts);
        self.index_buffer.update(&indices);

        for batch in batches {
            if let Some(sprite) = batch.sprite {
                if let Some(sprite) = self.resource_manager.get::<Sprite>(sprite) {
                    self.ctx.set_images(&[&sprite.image]);
                } else {
                    continue;
                }
            } else {
                self.ctx.set_images(&[&self.blank_image]);
            }
            self.ctx.draw(batch.start, batch.count, 1);
        }

        self.ctx.end_render_pass();
    }
}
