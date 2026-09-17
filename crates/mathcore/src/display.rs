//! The output of layout: a flat, platform-neutral display list.
//!
//! Coordinates are in pixels, y grows downward, and the origin is the top-left
//! corner of the formula's bounding box. The baseline of the outermost line
//! sits at `y = ascent`. Backends draw each glyph with the engine's math font
//! at the given em size and each rule as a filled rectangle.

/// An RGBA color with 8 bits per channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u8, pub u8, pub u8, pub u8);

impl Color {
    pub const BLACK: Color = Color(0, 0, 0, 255);
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Glyph {
        /// Which font of the engine's chain the glyph belongs to: 0 is the
        /// primary, 1 and up are its fallbacks.
        font: u16,
        /// Glyph index in that font.
        id: u16,
        /// Baseline origin of the glyph.
        x: f32,
        y: f32,
        /// Em size in pixels at which to draw the glyph.
        size: f32,
        color: Color,
    },
    Rule {
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Color,
    },
    /// A stroked straight line (used by `\cancel`).
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        thickness: f32,
        color: Color,
    },
}

/// Where a piece of the source ended up on screen, for hit testing and
/// selection. Regions nest: a tap usually lands in several, innermost last.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    /// Byte range of the source that produced this piece.
    pub start: u32,
    pub end: u32,
    /// Bounding box in the same pixel space as `Item`, y down from the top.
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Nesting level; 0 is a top-level atom of the formula.
    pub depth: u16,
}

impl Region {
    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    pub fn area(&self) -> f32 {
        self.width * self.height
    }

    /// Distance from the point to this rectangle; zero when inside.
    pub fn distance(&self, x: f32, y: f32) -> f32 {
        let dx = (self.x - x).max(x - (self.x + self.width)).max(0.0);
        let dy = (self.y - y).max(y - (self.y + self.height)).max(0.0);
        (dx * dx + dy * dy).sqrt()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayList {
    pub width: f32,
    /// Distance from the top of the bounding box to the baseline.
    pub ascent: f32,
    /// Distance from the baseline to the bottom of the bounding box.
    pub descent: f32,
    pub items: Vec<Item>,
    /// Source regions, outermost first. Empty unless hit testing was asked for.
    pub regions: Vec<Region>,
}

impl DisplayList {
    pub fn height(&self) -> f32 {
        self.ascent + self.descent
    }

    /// Every region containing the point, outermost first. A tap on the `a` of
    /// `\frac{a}{b}` returns the fraction and then the `a`, so a host can offer
    /// the whole sub-expression or the single symbol.
    pub fn hit(&self, x: f32, y: f32) -> Vec<Region> {
        let mut found: Vec<Region> = self.regions.iter().copied().filter(|r| r.contains(x, y)).collect();
        found.sort_by(|a, b| b.area().partial_cmp(&a.area()).unwrap_or(std::cmp::Ordering::Equal));
        found
    }

    /// The smallest piece of source under the point.
    pub fn hit_innermost(&self, x: f32, y: f32) -> Option<Region> {
        self.hit(x, y).pop()
    }

    /// The innermost region under the point, or the closest one when the point
    /// falls in the space between atoms. A tap is never pixel-exact, so this is
    /// what a widget should call.
    pub fn hit_nearest(&self, x: f32, y: f32) -> Option<Region> {
        if let Some(r) = self.hit_innermost(x, y) {
            return Some(r);
        }
        self.regions
            .iter()
            .min_by(|a, b| {
                let (da, db) = (a.distance(x, y), b.distance(x, y));
                da.partial_cmp(&db)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then(a.area().partial_cmp(&b.area()).unwrap())
            })
            .copied()
    }

    /// Rectangles covering a range of source, for a selection highlight. Only
    /// the outermost regions are returned, so the rectangles do not overlap.
    pub fn highlight(&self, start: u32, end: u32) -> Vec<Region> {
        let mut out: Vec<Region> = Vec::new();
        for r in self.regions.iter().filter(|r| r.start >= start && r.end <= end) {
            if out.iter().any(|o| o.start <= r.start && o.end >= r.end) {
                continue; // already covered by an enclosing region
            }
            out.retain(|o| !(r.start <= o.start && r.end >= o.end));
            out.push(*r);
        }
        out
    }
}

impl DisplayList {
    /// Packs the list into one flat `f32` buffer for FFI boundaries that
    /// cannot cheaply cross structured data (JNI, wasm-bindgen, dart:ffi):
    ///
    /// ```text
    /// [width, ascent, descent, count,
    ///  kind, glyph, x, y, w, h, thickness, colorBits,   <- item 0
    ///  ...]
    /// ```
    ///
    /// `kind` is 0 glyph (`w` = em size, `h` = font index), 1 rule (`w`,`h` = size), 2 line
    /// (`w`,`h` = x2,y2). `colorBits` is 0xAARRGGBB reinterpreted as a float.
    /// A region block follows the items, empty unless hit testing was on:
    ///
    /// ```text
    /// [regionCount, (start, end, x, y, width, height, depth) * regionCount]
    /// ```
    /// A region block follows the items:
    ///
    /// ```text
    /// [regionCount, start, end, x, y, width, height, depth, ...]
    /// ```
    pub fn to_flat(&self) -> Vec<f32> {
        fn argb_bits(c: Color) -> f32 {
            f32::from_bits(((c.3 as u32) << 24) | ((c.0 as u32) << 16) | ((c.1 as u32) << 8) | c.2 as u32)
        }
        let mut out = Vec::with_capacity(4 + self.items.len() * 8);
        out.extend([self.width, self.ascent, self.descent, self.items.len() as f32]);
        for it in &self.items {
            match *it {
                Item::Glyph {
                    font,
                    id,
                    x,
                    y,
                    size,
                    color,
                } => out.extend([0.0, id as f32, x, y, size, font as f32, 0.0, argb_bits(color)]),
                Item::Rule {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => out.extend([1.0, 0.0, x, y, width, height, 0.0, argb_bits(color)]),
                Item::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    thickness,
                    color,
                } => out.extend([2.0, 0.0, x1, y1, x2, y2, thickness, argb_bits(color)]),
            }
        }
        out.push(self.regions.len() as f32);
        for r in &self.regions {
            out.extend([r.start as f32, r.end as f32, r.x, r.y, r.width, r.height, r.depth as f32]);
        }
        out
    }
}
