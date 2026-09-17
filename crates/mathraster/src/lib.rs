//! Headless backends for `mathcore` display lists.
//!
//! `rasterize` paints into a `tiny_skia::Pixmap` (CPU, no GPU, no OS text
//! stack) and is what CI golden-image tests use. `to_svg` writes a compact
//! SVG that deduplicates glyph outlines through `<defs>`.
//!
//! Platform backends (Android Canvas, Core Graphics, Flutter, web canvas)
//! follow the same shape: walk `items`, draw glyphs by id at `size`, fill rules.

use mathcore::{Color, DisplayList, Item, MathFont};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Rect, Transform};
use ttf_parser::{GlyphId, OutlineBuilder};

pub use tiny_skia;

/// Rasterization settings.
#[derive(Debug, Clone)]
pub struct RasterOptions {
    /// Device pixel ratio. 1.0 maps one layout pixel to one image pixel.
    pub scale: f32,
    /// Padding around the formula, in layout pixels.
    pub padding: f32,
    /// Background color; `None` leaves the pixmap transparent.
    pub background: Option<Color>,
}

impl Default for RasterOptions {
    fn default() -> Self {
        RasterOptions {
            scale: 1.0,
            padding: 4.0,
            background: Some(Color(255, 255, 255, 255)),
        }
    }
}

struct SkiaOutline {
    pb: PathBuilder,
}

impl OutlineBuilder for SkiaOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.pb.move_to(x, y);
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.pb.line_to(x, y);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.pb.quad_to(x1, y1, x, y);
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.pb.cubic_to(x1, y1, x2, y2, x, y);
    }
    fn close(&mut self) {
        self.pb.close();
    }
}

fn glyph_path(font: &MathFont<'_>, id: u16) -> Option<tiny_skia::Path> {
    let mut b = SkiaOutline { pb: PathBuilder::new() };
    if !font.outline(GlyphId(id), &mut b) {
        return None;
    }
    b.pb.finish()
}

/// Paints the display list into a new pixmap.
pub fn rasterize(font: &MathFont<'_>, dl: &DisplayList, opts: &RasterOptions) -> Option<Pixmap> {
    let pad = opts.padding;
    let w = ((dl.width + 2.0 * pad) * opts.scale).ceil().max(1.0) as u32;
    let h = ((dl.height() + 2.0 * pad) * opts.scale).ceil().max(1.0) as u32;
    let mut pixmap = Pixmap::new(w, h)?;
    if let Some(bg) = opts.background {
        pixmap.fill(tiny_skia::Color::from_rgba8(bg.0, bg.1, bg.2, bg.3));
    }
    let upem = font.units_per_em();
    let mut cache: BTreeMap<u16, Option<tiny_skia::Path>> = BTreeMap::new();
    let mut paint = Paint {
        anti_alias: true,
        ..Default::default()
    };
    for item in &dl.items {
        match item {
            Item::Glyph { id, x, y, size, color } => {
                let path = cache.entry(*id).or_insert_with(|| glyph_path(font, *id));
                let Some(path) = path else { continue };
                paint.set_color_rgba8(color.0, color.1, color.2, color.3);
                let k = size / upem * opts.scale;
                let t = Transform::from_row(k, 0.0, 0.0, -k, (x + pad) * opts.scale, (y + pad) * opts.scale);
                pixmap.fill_path(path, &paint, FillRule::Winding, t, None);
            }
            Item::Rule {
                x,
                y,
                width,
                height,
                color,
            } => {
                paint.set_color_rgba8(color.0, color.1, color.2, color.3);
                if let Some(r) = Rect::from_xywh(
                    (x + pad) * opts.scale,
                    (y + pad) * opts.scale,
                    width * opts.scale,
                    height * opts.scale,
                ) {
                    pixmap.fill_rect(r, &paint, Transform::identity(), None);
                }
            }
        }
    }
    Some(pixmap)
}

/// Encodes the display list as PNG bytes.
pub fn to_png(font: &MathFont<'_>, dl: &DisplayList, opts: &RasterOptions) -> Option<Vec<u8>> {
    rasterize(font, dl, opts)?.encode_png().ok()
}

struct SvgOutline {
    d: String,
}

impl OutlineBuilder for SvgOutline {
    fn move_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.d, "M{x} {y}");
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let _ = write!(self.d, "L{x} {y}");
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let _ = write!(self.d, "Q{x1} {y1} {x} {y}");
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let _ = write!(self.d, "C{x1} {y1} {x2} {y2} {x} {y}");
    }
    fn close(&mut self) {
        self.d.push('Z');
    }
}

fn css_color(c: &Color) -> String {
    if c.3 == 255 {
        format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
    } else {
        format!("rgba({},{},{},{})", c.0, c.1, c.2, c.3 as f32 / 255.0)
    }
}

/// Serializes the display list as a standalone SVG document.
pub fn to_svg(font: &MathFont<'_>, dl: &DisplayList, padding: f32) -> String {
    let upem = font.units_per_em();
    let w = dl.width + 2.0 * padding;
    let h = dl.height() + 2.0 * padding;
    let mut defs = String::new();
    let mut body = String::new();
    let mut seen: BTreeMap<u16, bool> = BTreeMap::new();
    for item in &dl.items {
        match item {
            Item::Glyph { id, x, y, size, color } => {
                let has_outline = *seen.entry(*id).or_insert_with(|| {
                    let mut b = SvgOutline { d: String::new() };
                    if font.outline(GlyphId(*id), &mut b) && !b.d.is_empty() {
                        let _ = write!(defs, r##"<path id="g{id}" d="{}"/>"##, b.d);
                        true
                    } else {
                        false
                    }
                });
                if !has_outline {
                    continue;
                }
                let k = size / upem;
                let _ = write!(
                    body,
                    r##"<use href="#g{id}" transform="translate({} {}) scale({k} {})" fill="{}"/>"##,
                    x + padding,
                    y + padding,
                    -k,
                    css_color(color)
                );
            }
            Item::Rule {
                x,
                y,
                width,
                height,
                color,
            } => {
                let _ = write!(
                    body,
                    r#"<rect x="{}" y="{}" width="{width}" height="{height}" fill="{}"/>"#,
                    x + padding,
                    y + padding,
                    css_color(color)
                );
            }
        }
    }
    format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}"><defs>{defs}</defs>{body}</svg>"#)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mathcore::RenderOptions;

    const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");

    #[test]
    fn rasterizes_ink() {
        let font = MathFont::from_bytes(FONT).unwrap();
        let dl = mathcore::render(&font, r"\frac{1}{2}", &RenderOptions::default()).unwrap();
        let px = rasterize(&font, &dl, &RasterOptions::default()).unwrap();
        let dark = px.pixels().iter().filter(|p| p.red() < 128).count();
        assert!(dark > 50, "expected ink, got {dark} dark pixels");
    }

    #[test]
    fn svg_contains_defs_and_uses() {
        let font = MathFont::from_bytes(FONT).unwrap();
        let dl = mathcore::render(&font, "xx", &RenderOptions::default()).unwrap();
        let svg = to_svg(&font, &dl, 2.0);
        assert_eq!(svg.matches("<path id=").count(), 1, "one glyph outline shared");
        assert_eq!(svg.matches("<use ").count(), 2);
    }
}
