//! Types related to fonts and text rendering.

use std::collections::HashMap;

use etagere::euclid::Size2D;
use etagere::{AllocId, AtlasAllocator};
use fontdue::layout::{GlyphPosition, GlyphRasterConfig, Layout, LayoutSettings, TextStyle};

use crate::assets::ResourceHandle;
use crate::graphics::{Color, Graphics, Sprite};

const ATLAS_SIZE: u32 = 2048;

/// A TrueType/OpenType font, owning an immutable copy of the font data.
pub struct Font {
    layout: Layout,
    inner: FontInner,
}

struct FontInner {
    font: fontdue::Font,
    sprites: Vec<ResourceHandle<Sprite>>,
    allocators: Vec<AtlasAllocator>,
    glyphs: HashMap<GlyphRasterConfig, Option<(usize, AllocId)>>,
    draw_commands: Vec<DrawCommand>,
}

impl Font {
    /// Creates a new font from the given data.
    pub fn new(data: impl AsRef<[u8]>) -> Self {
        Self {
            layout: Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown),
            inner: FontInner {
                font: fontdue::Font::from_bytes(data.as_ref(), Default::default()).unwrap(),
                sprites: Vec::new(),
                allocators: Vec::new(),
                glyphs: HashMap::new(),
                draw_commands: Vec::new(),
            },
        }
    }
}

struct DrawCommand {
    x: f32,
    y: f32,
    sx: f32,
    sy: f32,
    sw: f32,
    sh: f32,
    sprite: usize,
}

pub(crate) fn draw_text(
    g: &mut Graphics,
    x: f32,
    y: f32,
    text: &str,
    font: ResourceHandle<Font>,
    size: f32,
    color: Color,
) {
    if let Some(mut font) = g.resource_manager.get(font) {
        let Font { layout, inner } = &mut *font;

        layout.reset(&LayoutSettings {
            x,
            y,
            ..Default::default()
        });
        layout.append(
            std::slice::from_ref(&inner.font),
            &TextStyle::new(text, size, 0),
        );

        inner.draw_commands.clear();
        for glyph in layout.glyphs() {
            draw_char(g, glyph, inner, size);
        }

        if inner.sprites.len() > 1 {
            inner
                .draw_commands
                .sort_unstable_by(|a, b| usize::cmp(&a.sprite, &b.sprite));
        }

        for cmd in &inner.draw_commands {
            g.draw_sprite(cmd.x, cmd.y, inner.sprites[cmd.sprite])
                .source_rect(cmd.sx, cmd.sy, cmd.sw, cmd.sh)
                .color(color);
        }
    }
}

fn draw_char(
    g: &mut Graphics,
    glyph: &GlyphPosition,
    FontInner {
        font,
        sprites,
        allocators,
        glyphs,
        draw_commands,
    }: &mut FontInner,
    size: f32,
) {
    fn push_atlas(
        g: &mut Graphics,
        sprites: &mut Vec<ResourceHandle<Sprite>>,
        allocators: &mut Vec<AtlasAllocator>,
    ) {
        let sprite = g.resource_manager.allocate();
        g.resource_manager.set(
            sprite,
            Sprite::new(
                &g.ctx,
                ATLAS_SIZE,
                ATLAS_SIZE,
                fugu::ImageFormat::Rgba8,
                fugu::ImageFilter::Linear,
                fugu::ImageWrap::Clamp,
                &vec![0; ATLAS_SIZE as usize * ATLAS_SIZE as usize * 4],
            ),
        );
        sprites.push(sprite);
        allocators.push(AtlasAllocator::new(Size2D::new(
            ATLAS_SIZE as _,
            ATLAS_SIZE as _,
        )));
    }

    if sprites.is_empty() {
        push_atlas(g, sprites, allocators);
    }

    let entry = glyphs.entry(glyph.key).or_insert_with(|| {
        let c = glyph.parent;
        let (metrics, data) = glyph
            .char_data
            .rasterize()
            .then(|| font.rasterize(c, size))?;
        if metrics.width == 0 || metrics.height == 0 {
            None
        } else if metrics.width > ATLAS_SIZE as _ || metrics.height > ATLAS_SIZE as _ {
            panic!("glyph bigger than atlas");
        } else {
            // TODO: maybe use a heuristic to optimize choosing which atlas to use
            let (i, alloc) = allocators
                .iter_mut()
                .enumerate()
                .find_map(|(i, e)| {
                    e.allocate(Size2D::new(metrics.width as _, metrics.height as _))
                        .map(|alloc| (i, alloc))
                })
                .unwrap_or_else(|| {
                    let i = allocators.len();
                    push_atlas(g, sprites, allocators);
                    let alloc = allocators[i]
                        .allocate(Size2D::new(metrics.width as _, metrics.height as _))
                        .unwrap();
                    (i, alloc)
                });
            let data = data
                .into_iter()
                .flat_map(|x| [255, 255, 255, x])
                .collect::<Vec<_>>();
            g.resource_manager
                .get(sprites[i])
                .unwrap()
                .inner()
                .update_part(
                    alloc.rectangle.min.x as _,
                    alloc.rectangle.min.y as _,
                    metrics.width as _,
                    metrics.height as _,
                    &data,
                );
            Some((i, alloc.id))
        }
    });
    if let &mut Some((i, id)) = entry {
        let rect = allocators[i].get(id);
        draw_commands.push(DrawCommand {
            x: glyph.x,
            y: glyph.y,
            sx: rect.min.x as _,
            sy: rect.min.y as _,
            sw: rect.size().width as _,
            sh: rect.size().height as _,
            sprite: i,
        });
    }
}
