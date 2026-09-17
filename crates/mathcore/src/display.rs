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
