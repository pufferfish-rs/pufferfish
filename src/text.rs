//! Types related to fonts and text rendering.

use std::cell::RefCell;
use std::collections::HashMap;

use etagere::euclid::Size2D;
use etagere::{AllocId, AtlasAllocator};
use fontdue::layout::{GlyphPosition, GlyphRasterConfig, Layout, LayoutSettings, TextStyle};
use fontdue::Metrics;

use crate::assets::ResourceHandle;
use crate::graphics::{Color, Graphics, Sprite};

const ATLAS_SIZE: u32 = 2048;

/// A TrueType/OpenType font, owning an immutable copy of the font data.
pub struct Font {
    layout: RefCell<Layout>,
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
            layout: RefCell::new(Layout::new(
                fontdue::layout::CoordinateSystem::PositiveYDown,
            )),
            inner: FontInner {
                font: fontdue::Font::from_bytes(data.as_ref(), Default::default()).unwrap(),
                sprites: Vec::new(),
                allocators: Vec::new(),
                glyphs: HashMap::new(),
                draw_commands: Vec::new(),
            },
        }
    }

    /// Returns the metrics of the given glyph in the given font or `None`
    /// if the given glyph does not exist.
    pub fn measure_glyph(&self, c: char, size: f32) -> Option<GlyphMetrics> {
        let font = &self.inner.font;
        let metrics = font.metrics_indexed(font.chars().get(&c)?.get(), size);

        Some(GlyphMetrics {
            x: metrics.xmin,
            y: metrics.ymin,
            width: metrics.width as u32,
            height: metrics.height as u32,
            advance: metrics.advance_width,
        })
    }

    /// Measures and returns the width and height of the given text in the given
    /// font.
    pub fn measure_text(&self, text: &str, size: f32) -> (f32, f32) {
        let Font { layout, inner } = self;
        let mut layout = layout.borrow_mut();

        layout.reset(&LayoutSettings::default());
        layout.append(
            std::slice::from_ref(&inner.font),
            &TextStyle::new(text, size, 0),
        );

        let (x_min, x_max) = layout
            .glyphs()
            .iter()
            .fold((0_f32, 0_f32), |(min, max), glyph| {
                (min.min(glyph.x), max.max(glyph.x + glyph.width as f32))
            });

        (x_max - x_min, layout.height())
    }

    /// Returns the metrics of the given font.
    pub fn measure_font(&self, size: f32) -> FontMetrics {
        let metrics = self
            .inner
            .font
            .horizontal_line_metrics(size)
            .expect("vertical fonts are not yet supported");
        FontMetrics {
            ascent: metrics.ascent,
            descent: metrics.descent,
            line_gap: metrics.line_gap,
            line_height: metrics.new_line_size,
        }
    }

    /// Returns the kerning between the given glyphs in the given font or `None`
    /// if a kerning value does not exist between the given pair of glyphs.
    pub fn measure_kern(&self, left: char, right: char, size: f32) -> Option<f32> {
        self.inner.font.horizontal_kern(left, right, size)
    }
}

/// Font metrics. Returned by the [`measure_font`] method on [`Graphics`].
///
/// [`measure_font`]: crate::graphics::Graphics::measure_font
pub struct FontMetrics {
    /// A typically positive number representing highest point a glyph in the
    /// font extends above the baseline.
    pub ascent: f32,
    /// A typically negative number representing highest point a glyph in the
    /// font extends below the baseline.
    pub descent: f32,
    /// The recommended size of the gap between the descent of one line and the
    /// ascent of the next line.
    pub line_gap: f32,
    /// The recommended total line height. Equivalent to `ascent + descent +
    /// line_gap`.
    pub line_height: f32,
}

/// Glyph metrics. Returned by the [`measure_glyph`] method on [`Graphics`].
///
/// [`measure_glyph`]: crate::graphics::Graphics::measure_glyph
pub struct GlyphMetrics {
    /// The horizontal offset of the glyph bitmap relative to the origin.
    pub x: i32,
    /// The vertical offset of the glyph bitmap relative to the origin.
    pub y: i32,
    /// The width of the glyph bitmap.
    pub width: u32,
    /// The height of the glyph bitmap.
    pub height: u32,
    /// The advance width of the glyph.
    pub advance: f32,
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_text(
    g: &mut Graphics,
    x: f32,
    y: f32,
    text: &str,
    font: ResourceHandle<Font>,
    size: f32,
    color: Color,
    depth: f32,
) {
    if let Some(mut font) = g.resource_manager.get(font) {
        let Font { layout, inner } = &mut *font;
        let mut layout = layout.borrow_mut();

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
                .color(color)
                .depth(depth);
        }
    }
}

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

fn insert_glyph(
    g: &mut Graphics,
    metrics: &Metrics,
    data: &[u8],
    sprites: &mut Vec<ResourceHandle<Sprite>>,
    allocators: &mut Vec<AtlasAllocator>,
) -> Option<(usize, AllocId)> {
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
            .iter()
            .flat_map(|&x| [255, 255, 255, x])
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
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_glyph(
    g: &mut Graphics,
    x: f32,
    y: f32,
    c: char,
    font: ResourceHandle<Font>,
    size: f32,
    color: Color,
    depth: f32,
) {
    if let Some(mut font) = g.resource_manager.get(font) {
        let FontInner {
            font,
            sprites,
            allocators,
            glyphs,
            ..
        } = &mut font.inner;

        if sprites.is_empty() {
            push_atlas(g, sprites, allocators);
        }

        let key = GlyphRasterConfig {
            glyph_index: font.lookup_glyph_index(c),
            px: size,
            font_hash: font.file_hash(),
        };

        let entry = glyphs.entry(key).or_insert_with(|| {
            let (metrics, data) = font.rasterize(c, size);
            insert_glyph(g, &metrics, &data, sprites, allocators)
        });

        if let &mut Some((i, id)) = entry {
            let rect = allocators[i].get(id);
            g.draw_sprite(x, y, sprites[i])
                .source_rect(
                    rect.min.x as _,
                    rect.min.y as _,
                    rect.size().width as _,
                    rect.size().height as _,
                )
                .color(color)
                .depth(depth);
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
    if sprites.is_empty() {
        push_atlas(g, sprites, allocators);
    }

    let entry = glyphs.entry(glyph.key).or_insert_with(|| {
        let c = glyph.parent;
        let (metrics, data) = glyph
            .char_data
            .rasterize()
            .then(|| font.rasterize(c, size))?;
        insert_glyph(g, &metrics, &data, sprites, allocators)
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
