//! Types related to fonts and text rendering.

use std::collections::HashMap;

use etagere::euclid::Size2D;
use etagere::{AllocId, AtlasAllocator};
use fontdue::layout::{Layout, TextStyle};

use crate::assets::ResourceHandle;
use crate::graphics::{Graphics, Sprite};

struct FontAtlas {
    sprite: ResourceHandle<Sprite>,
    allocator: AtlasAllocator,
    glyphs: HashMap<char, Option<AllocId>>,
}

/// A TrueType/OpenType font, owning an immutable copy of the font data.
pub struct Font {
    inner: fontdue::Font,
    layout: Layout,
    atlases: HashMap<u32, FontAtlas>,
}

impl Font {
    /// Creates a new font from the given data.
    pub fn new(data: impl AsRef<[u8]>) -> Self {
        Self {
            inner: fontdue::Font::from_bytes(data.as_ref(), Default::default()).unwrap(),
            layout: Layout::new(fontdue::layout::CoordinateSystem::PositiveYDown),
            atlases: HashMap::new(),
        }
    }
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
            atlases,
        } = &mut *font;
        layout.clear();
        layout.append(std::slice::from_ref(inner), &TextStyle::new(text, size, 0));
        for glyph in layout.glyphs() {
            draw_char(
                g,
                x + glyph.x,
                y + glyph.y,
                glyph.parent,
                inner,
                atlases,
                size,
            );
        }
    }
}

fn draw_char(
    g: &mut Graphics,
    x: f32,
    y: f32,
    c: char,
    inner: &mut fontdue::Font,
    atlases: &mut HashMap<u32, FontAtlas>,
    size: f32,
) {
    let key = size.to_bits();
    // let Font { inner, atlases, .. } = &mut *font;
    let atlas = atlases.entry(key).or_insert_with(|| {
        let sprite = g.resource_manager.allocate();
        g.resource_manager.set(
            sprite,
            Sprite::new(
                &g.ctx,
                2048,
                2048,
                fugu::ImageFormat::Rgba8,
                fugu::ImageFilter::Linear,
                fugu::ImageWrap::Clamp,
                &vec![0; 2048 * 2048 * 4],
            ),
        );
        let allocator = AtlasAllocator::new(Size2D::new(2048, 2048));
        let glyphs = HashMap::new();
        FontAtlas {
            sprite,
            allocator,
            glyphs,
        }
    });
    let id = atlas.glyphs.entry(c).or_insert_with(|| {
        let (metrics, data) = inner.rasterize(c, size);
        if metrics.width == 0 || metrics.height == 0 || c == '\n' {
            None
        } else {
            let alloc = atlas
                .allocator
                .allocate(Size2D::new(metrics.width as _, metrics.height as _))
                .unwrap();
            let data = data
                .into_iter()
                .flat_map(|x| [255, 255, 255, x])
                .collect::<Vec<_>>();
            g.resource_manager
                .get(atlas.sprite)
                .unwrap()
                .inner()
                .update_part(
                    alloc.rectangle.min.x as _,
                    alloc.rectangle.min.y as _,
                    metrics.width as _,
                    metrics.height as _,
                    &data,
                );
            Some(alloc.id)
        }
    });
    if let Some(id) = id {
        let rect = atlas.allocator.get(*id);
        g.draw_sprite_part(
            x,
            y,
            rect.min.x as f32,
            rect.min.y as f32,
            rect.size().width as f32,
            rect.size().height as f32,
            atlas.sprite,
        );
    }
}
