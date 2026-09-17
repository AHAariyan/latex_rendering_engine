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
        /// Glyph index in the engine's math font.
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DisplayList {
    pub width: f32,
    /// Distance from the top of the bounding box to the baseline.
    pub ascent: f32,
    /// Distance from the baseline to the bottom of the bounding box.
    pub descent: f32,
    pub items: Vec<Item>,
}

impl DisplayList {
    pub fn height(&self) -> f32 {
        self.ascent + self.descent
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
    /// `kind` is 0 glyph (`w` = em size), 1 rule (`w`,`h` = size), 2 line
    /// (`w`,`h` = x2,y2). `colorBits` is 0xAARRGGBB reinterpreted as a float.
    pub fn to_flat(&self) -> Vec<f32> {
        fn argb_bits(c: Color) -> f32 {
            f32::from_bits(((c.3 as u32) << 24) | ((c.0 as u32) << 16) | ((c.1 as u32) << 8) | c.2 as u32)
        }
        let mut out = Vec::with_capacity(4 + self.items.len() * 8);
        out.extend([self.width, self.ascent, self.descent, self.items.len() as f32]);
        for it in &self.items {
            match *it {
                Item::Glyph { id, x, y, size, color } => out.extend([0.0, id as f32, x, y, size, 0.0, 0.0, argb_bits(color)]),
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
        out
    }
}
