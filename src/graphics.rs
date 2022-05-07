use fugu::{
    Buffer, BufferKind, BufferLayout, BufferUsage, Context, Image, ImageFilter, ImageFormat,
    ImageUniform, ImageWrap, PassAction, Pipeline, Uniform, UniformFormat, VertexAttribute,
    VertexFormat,
};

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

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub fn from_rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    pub fn from_rgb(r: f32, g: f32, b: f32) -> Color {
        Color { r, g, b, a: 1. }
    }
}

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Debug)]
struct Vertex {
    pos: (f32, f32),
    color: Color,
    uv: (f32, f32),
}

struct DrawCommand {
    verts: Vec<Vertex>,
    indices: Vec<u16>,
}

pub struct Graphics {
    pub ctx: Context,
    pipeline: Pipeline,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    blank_image: Image,
    draw_commands: Vec<DrawCommand>,
    viewport: (f32, f32),
    color: Color,
}

impl Graphics {
    pub(crate) fn new(ctx: Context) -> Graphics {
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
        let draw_commands = Vec::new();
        let viewport = (0., 0.);
        let color = Color::from_rgb(1., 1., 1.);

        Graphics {
            ctx,
            pipeline,
            vertex_buffer,
            index_buffer,
            blank_image,
            draw_commands,
            viewport,
            color,
        }
    }

    pub(crate) fn set_viewport(&mut self, (width, height): (u32, u32)) {
        self.ctx.set_viewport(0, 0, width, height);
        self.viewport = (width as f32, height as f32);
    }

    pub fn clear(&self, color: Color) {
        self.ctx.begin_default_pass(PassAction::Clear {
            color: Some((color.r, color.g, color.b, color.a)),
            depth: None,
            stencil: None,
        });
        self.ctx.end_render_pass();
    }

    pub fn begin(&mut self) {}

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }

    pub fn draw_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.draw_commands.push(DrawCommand {
            verts: vec![
                Vertex {
                    pos: (x, y),
                    color: self.color,
                    uv: (0., 0.),
                },
                Vertex {
                    pos: (x + w, y),
                    color: self.color,
                    uv: (1., 0.),
                },
                Vertex {
                    pos: (x + w, y + h),
                    color: self.color,
                    uv: (1., 1.),
                },
                Vertex {
                    pos: (x, y + h),
                    color: self.color,
                    uv: (0., 1.),
                },
            ],
            indices: vec![0, 3, 1, 1, 3, 2],
        });
    }

    pub fn end(&mut self) {
        self.ctx.begin_default_pass(PassAction::Nothing);

        self.ctx.set_pipeline(&self.pipeline);
        self.ctx.set_vertex_buffer(&self.vertex_buffer);
        self.ctx.set_index_buffer(&self.index_buffer);
        self.ctx.set_uniforms(self.viewport);
        self.ctx.set_images(&[&self.blank_image]);

        let mut verts = Vec::new();
        let mut indices = Vec::new();

        for draw_command in &self.draw_commands {
            indices.extend(
                draw_command
                    .indices
                    .iter()
                    .copied()
                    .map(|e| e + verts.len() as u16),
            );
            verts.extend_from_slice(&draw_command.verts);
        }
        self.draw_commands.clear();

        self.vertex_buffer.update(&verts);
        self.index_buffer.update(&indices);

        self.ctx.draw(0, indices.len(), 1);

        self.ctx.end_render_pass();
    }
}
