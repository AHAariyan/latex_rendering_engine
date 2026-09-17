//! WebAssembly binding.
//!
//! `MathEngine.renderSvg` returns a standalone SVG string, which is the
//! simplest way to put a formula in the DOM. `MathEngine.render` returns the
//! same flat `Float32Array` layout as the Android binding, for drawing on a
//! `<canvas>` with cached `Path2D` outlines (see `platforms/web/mathcore.js`).

use mathcore::{Color, LineBreak, Macros, MathFont, RenderOptions};
use ttf_parser::OutlineBuilder;
use wasm_bindgen::prelude::*;

const BUNDLED_FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math-subset.otf");

#[wasm_bindgen]
pub struct MathEngine {
    font: MathFont<'static>,
}

fn color_from_argb(argb: u32) -> Color {
    Color((argb >> 16) as u8, (argb >> 8) as u8, argb as u8, (argb >> 24) as u8)
}

fn macros_from(text: Option<String>) -> Macros {
    let mut m = Macros::new();
    if let Some(t) = text {
        for line in t.lines() {
            if let Some((name, body)) = line.split_once('=') {
                m.define(name.trim(), body);
            }
        }
    }
    m
}

#[wasm_bindgen]
impl MathEngine {
    /// Engine with the bundled Latin Modern Math font.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<MathEngine, JsError> {
        let font = MathFont::from_bytes(BUNDLED_FONT).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(MathEngine { font })
    }

    /// Engine for any OpenType font with a MATH table. The bytes are copied and kept for the page lifetime.
    #[wasm_bindgen(js_name = fromFont)]
    pub fn from_font(bytes: &[u8]) -> Result<MathEngine, JsError> {
        let leaked: &'static [u8] = Box::leak(bytes.to_vec().into_boxed_slice());
        let font = MathFont::from_bytes(leaked).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(MathEngine { font })
    }

    #[wasm_bindgen(js_name = unitsPerEm)]
    pub fn units_per_em(&self) -> f32 {
        self.font.units_per_em()
    }

    /// Renders to a standalone SVG string. `argb` is 0xAARRGGBB; `macros` is
    /// newline-separated `\name=body`; `maxWidth` of 0 renders one line.
    #[wasm_bindgen(js_name = renderSvg)]
    pub fn render_svg(
        &self,
        tex: &str,
        font_size: f32,
        display_mode: bool,
        argb: u32,
        macros: Option<String>,
        max_width: Option<f32>,
    ) -> Result<String, JsError> {
        let opts = RenderOptions {
            font_size,
            display_mode,
            color: color_from_argb(argb),
            macros: macros_from(macros),
            line_break: max_width.filter(|w| *w > 0.0).map(LineBreak::new),
        };
        let dl = mathcore::render(&self.font, tex, &opts).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(mathraster::to_svg(&self.font, &dl, 0.0))
    }

    /// Renders to the flat layout `[width, ascent, descent, count, (kind, glyph, x, y, w, h, thickness, colorBits) * count]`.
    pub fn render(
        &self,
        tex: &str,
        font_size: f32,
        display_mode: bool,
        argb: u32,
        macros: Option<String>,
        max_width: Option<f32>,
    ) -> Result<Vec<f32>, JsError> {
        let opts = RenderOptions {
            font_size,
            display_mode,
            color: color_from_argb(argb),
            macros: macros_from(macros),
            line_break: max_width.filter(|w| *w > 0.0).map(LineBreak::new),
        };
        let dl = mathcore::render(&self.font, tex, &opts).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(dl.to_flat())
    }

    /// Glyph outline in font units, y up: `0 x y` move, `1 x y` line, `2 x1 y1 x y` quad, `3 x1 y1 x2 y2 x y` cubic, `4` close.
    #[wasm_bindgen(js_name = glyphOutline)]
    pub fn glyph_outline(&self, glyph: u16) -> Option<Vec<f32>> {
        struct Stream(Vec<f32>);
        impl OutlineBuilder for Stream {
            fn move_to(&mut self, x: f32, y: f32) {
                self.0.extend([0.0, x, y]);
            }
            fn line_to(&mut self, x: f32, y: f32) {
                self.0.extend([1.0, x, y]);
            }
            fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
                self.0.extend([2.0, x1, y1, x, y]);
            }
            fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
                self.0.extend([3.0, x1, y1, x2, y2, x, y]);
            }
            fn close(&mut self) {
                self.0.push(4.0);
            }
        }
        let mut s = Stream(Vec::new());
        if self.font.outline(ttf_parser::GlyphId(glyph), &mut s) && !s.0.is_empty() {
            Some(s.0)
        } else {
            None
        }
    }
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
