//! Types related to fonts and text rendering.

use std::collections::HashMap;

use etagere::euclid::Size2D;
use etagere::{AllocId, AtlasAllocator};
use fontdue::layout::{GlyphPosition, GlyphRasterConfig, Layout, LayoutSettings, TextStyle};

use crate::assets::ResourceHandle;
use crate::graphics::{Graphics, Sprite};

const ATLAS_SIZE: u32 = 2048;

/// A TrueType/OpenType font, owning an immutable copy of the font data.
pub struct Font {
    inner: fontdue::Font,
    layout: Layout,
    sprites: Vec<ResourceHandle<Sprite>>,
    allocators: Vec<AtlasAllocator>,
    glyphs: HashMap<GlyphRasterConfig, Option<(usize, AllocId)>>,
    draw_commands: Vec<DrawCommand>,
}

impl Font {
    /// Creates a new font from the given data.
    pub fn new(data: impl AsRef<[u8]>) -> Self {
        Self {
            inner: fontdue::Font::from_bytes(data.as_ref(), Default::default()).unwrap(),
            layout: Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown),
            sprites: Vec::new(),
            allocators: Vec::new(),
            glyphs: HashMap::new(),
            draw_commands: Vec::new(),
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
) {
    if let Some(mut font) = g.resource_manager.get(font) {
        let Font {
            layout,
            inner,
            sprites,
            allocators,
            glyphs,
            draw_commands,
        } = &mut *font;

        layout.reset(&LayoutSettings {
            x,
            y,
            ..Default::default()
        });
        layout.append(std::slice::from_ref(inner), &TextStyle::new(text, size, 0));

        draw_commands.clear();
        for glyph in layout.glyphs() {
            draw_char(
                g,
                glyph,
                inner,
                sprites,
                allocators,
                glyphs,
                draw_commands,
                size,
            );
        }

        if sprites.len() > 1 {
            draw_commands.sort_unstable_by(|a, b| usize::cmp(&a.sprite, &b.sprite));
        }

        for cmd in draw_commands {
            g.draw_sprite_part(
                cmd.x,
                cmd.y,
                cmd.sx,
                cmd.sy,
                cmd.sw,
                cmd.sh,
                sprites[cmd.sprite],
            );
        }
    }
}

fn draw_char(
    g: &mut Graphics,
    glyph: &GlyphPosition,
    inner: &mut fontdue::Font,
    sprites: &mut Vec<ResourceHandle<Sprite>>,
    allocators: &mut Vec<AtlasAllocator>,
    glyphs: &mut HashMap<GlyphRasterConfig, Option<(usize, AllocId)>>,
    draw_commands: &mut Vec<DrawCommand>,
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
        let mut i = sprites.len() - 1;
        let (metrics, data) = inner.rasterize(c, size);
        if !glyph.char_data.rasterize() || metrics.width == 0 || metrics.height == 0 {
            None
        } else if metrics.width > ATLAS_SIZE as _ || metrics.height > ATLAS_SIZE as _ {
            panic!("glyph bigger than atlas");
        } else {
            // TODO: be smarter about choosing the atlas instead of just using the last one
            let alloc = allocators[i]
                .allocate(Size2D::new(metrics.width as _, metrics.height as _))
                .unwrap_or_else(|| {
                    push_atlas(g, sprites, allocators);
                    i += 1;
                    allocators[i]
                        .allocate(Size2D::new(metrics.width as _, metrics.height as _))
                        .unwrap()
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
