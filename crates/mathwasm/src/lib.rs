//! WebAssembly binding.
//!
//! `MathEngine.renderSvg` returns a standalone SVG string, which is the
//! simplest way to put a formula in the DOM. `MathEngine.render` returns the
//! same flat `Float32Array` layout as the Android binding, for drawing on a
//! `<canvas>` with cached `Path2D` outlines (see `platforms/web/mathcore.js`).

use mathcore::{Color, LineBreak, Macros, MathFont, RenderOptions};
use ttf_parser::OutlineBuilder;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct MathEngine {
    font: MathFont<'static>,
    cache: mathcore::LayoutCache,
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
        let font = mathcore::bundled::font().map_err(|e| JsError::new(&e.to_string()))?;
        Ok(MathEngine {
            font,
            cache: mathcore::LayoutCache::default(),
        })
    }

    /// Engine for any OpenType font with a MATH table. The bytes are copied and kept for the page lifetime.
    #[wasm_bindgen(js_name = fromFont)]
    pub fn from_font(bytes: &[u8]) -> Result<MathEngine, JsError> {
        let leaked: &'static [u8] = Box::leak(bytes.to_vec().into_boxed_slice());
        let font = MathFont::from_bytes(leaked).map_err(|e| JsError::new(&e.to_string()))?;
        let fallback = MathFont::from_bytes(mathcore::bundled::FALLBACK).map_err(|e| JsError::new(&e.to_string()))?;
        Ok(MathEngine {
            font: font.with_fallback(fallback),
            cache: mathcore::LayoutCache::default(),
        })
    }

    /// Adds a text font (Noto Sans Bengali, an Arabic or CJK face...) to the
    /// end of the chain for characters no earlier font has. `index` picks the
    /// face in a collection. The bytes are copied and kept for the page
    /// lifetime. Returns the font's index in the chain.
    #[wasm_bindgen(js_name = addFont)]
    pub fn add_font(&mut self, bytes: &[u8], index: Option<u32>) -> Result<u32, JsError> {
        let leaked: &'static [u8] = Box::leak(bytes.to_vec().into_boxed_slice());
        let font = MathFont::from_text_bytes(leaked, index.unwrap_or(0)).map_err(|e| JsError::new(&e.to_string()))?;
        let i = self.font.add_fallback(font);
        self.cache.clear();
        Ok(i as u32)
    }

    /// The characters of `tex` no font in the chain can draw, as a string
    /// ("" when every character is covered).
    #[wasm_bindgen(js_name = missingChars)]
    pub fn missing_chars(&self, tex: &str, display_mode: bool, macros: Option<String>) -> Result<String, JsError> {
        let opts = RenderOptions {
            display_mode,
            macros: macros_from(macros),
            ..RenderOptions::default()
        };
        mathcore::missing_chars(&self.font, tex, &opts)
            .map(|c| c.into_iter().collect())
            .map_err(|e| JsError::new(&e.to_string()))
    }

    /// Font units per em of one font of the chain; a fallback may differ.
    #[wasm_bindgen(js_name = unitsPerEm)]
    pub fn units_per_em(&self, font: Option<u16>) -> f32 {
        let i = font.unwrap_or(0) as usize;
        if i > self.font.fallback_count() {
            0.0
        } else {
            self.font.font_at(i).units_per_em()
        }
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
            budget: mathcore::Budget::default(),
            hit_testing: false,
        };
        let dl = self
            .cache
            .render(&self.font, tex, &opts)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(mathraster::to_svg(&self.font, &dl, 0.0))
    }

    /// Renders to the flat layout `[width, ascent, descent, count, (kind, glyph,
    /// x, y, w, h, thickness, colorBits) * count, regionCount, (start, end, x,
    /// y, w, h, depth) * regionCount]`. Regions are empty unless `hitTesting`.
    #[allow(clippy::too_many_arguments)] // one JavaScript-facing entry point
    pub fn render(
        &self,
        tex: &str,
        font_size: f32,
        display_mode: bool,
        argb: u32,
        macros: Option<String>,
        max_width: Option<f32>,
        hit_testing: Option<bool>,
    ) -> Result<Vec<f32>, JsError> {
        let opts = RenderOptions {
            font_size,
            display_mode,
            color: color_from_argb(argb),
            macros: macros_from(macros),
            line_break: max_width.filter(|w| *w > 0.0).map(LineBreak::new),
            budget: mathcore::Budget::default(),
            hit_testing: hit_testing.unwrap_or(false),
        };
        let dl = self
            .cache
            .render(&self.font, tex, &opts)
            .map_err(|e| JsError::new(&e.to_string()))?;
        Ok(dl.to_flat())
    }

    /// Glyph outline in font units, y up: `0 x y` move, `1 x y` line, `2 x1 y1 x y` quad, `3 x1 y1 x2 y2 x y` cubic, `4` close.
    #[wasm_bindgen(js_name = glyphOutline)]
    pub fn glyph_outline(&self, font: u16, glyph: u16) -> Option<Vec<f32>> {
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
        if font as usize > self.font.fallback_count() {
            return None;
        }
        let mut s = Stream(Vec::new());
        if self.font.font_at(font as usize).outline(ttf_parser::GlyphId(glyph), &mut s) && !s.0.is_empty() {
            Some(s.0)
        } else {
            None
        }
    }
}

/// Presentation MathML for a formula, for a screen reader. Needs no font.
/// The languages spoken math is available in, comma-separated BCP 47 tags.
#[wasm_bindgen(js_name = speechLanguages)]
pub fn speech_languages() -> String {
    let tags: Vec<&str> = mathcore::Language::ALL.iter().map(|l| l.tag()).collect();
    tags.join(",")
}

#[wasm_bindgen]
pub fn mathml(tex: &str, display_mode: bool, macros: Option<String>) -> Result<String, JsError> {
    mathcore::render_mathml(tex, display_mode, &macros_from(macros)).map_err(|e| JsError::new(&e.to_string()))
}

/// A spoken sentence for a formula, for an `aria-label`. Needs no font.
#[wasm_bindgen]
pub fn speech(tex: &str, macros: Option<String>) -> Result<String, JsError> {
    mathcore::render_speech(tex, &macros_from(macros)).map_err(|e| JsError::new(&e.to_string()))
}

fn speech_opts(verbosity: u8, language: Option<String>) -> mathcore::SpeechOptions {
    let verbosity = match verbosity {
        0 => mathcore::Verbosity::Verbose,
        2 => mathcore::Verbosity::Superbrief,
        _ => mathcore::Verbosity::Brief,
    };
    mathcore::SpeechOptions {
        verbosity,
        language: mathcore::Language::from_tag(language.as_deref().unwrap_or("")),
    }
}

/// `speech` at a verbosity (0 verbose, 1 brief, 2 superbrief) and in a
/// language (a BCP 47 tag: "es", "fr", "de", "pt", "bn", "hi"; English otherwise).
#[wasm_bindgen(js_name = speechWith)]
pub fn speech_with(tex: &str, verbosity: u8, macros: Option<String>, language: Option<String>) -> Result<String, JsError> {
    mathcore::render_speech_with(tex, &macros_from(macros), &speech_opts(verbosity, language)).map_err(|e| JsError::new(&e.to_string()))
}

/// The navigable speech tree as JSON; `JSON.parse` it. Each node's
/// `start..end` is a source byte range for highlighting.
#[wasm_bindgen(js_name = speechTree)]
pub fn speech_tree(tex: &str, verbosity: u8, macros: Option<String>, language: Option<String>) -> Result<String, JsError> {
    mathcore::render_speech_tree(tex, &macros_from(macros), &speech_opts(verbosity, language))
        .map(|n| n.to_json())
        .map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn nemeth(tex: &str, macros: Option<String>) -> Result<String, JsError> {
    mathcore::render_nemeth(tex, &macros_from(macros)).map_err(|e| JsError::new(&e.to_string()))
}

/// AsciiMath (`sum_(i=1)^n i^2`) translated to TeX for the other calls.
#[wasm_bindgen(js_name = asciimathToTex)]
pub fn asciimath_to_tex(src: &str) -> Result<String, JsError> {
    mathcore::asciimath_to_tex(src).map_err(|e| JsError::new(&e.to_string()))
}

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// A math input field's model: typing, keys, taps, caret and selection.
/// Draw it with `renderSvg`, then place a caret at `caret()`.
#[wasm_bindgen]
pub struct MathEditor {
    editor: mathcore::Editor,
    last: Option<mathcore::EditorLayout>,
}

impl Default for MathEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl MathEditor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> MathEditor {
        MathEditor {
            editor: mathcore::Editor::new(),
            last: None,
        }
    }

    /// Replaces the content with `tex`, cursor at the end.
    #[wasm_bindgen(js_name = setTex)]
    pub fn set_tex(&mut self, tex: &str) {
        self.editor = mathcore::Editor::from_tex(tex);
    }

    /// The content as TeX.
    pub fn tex(&self) -> String {
        self.editor.tex()
    }

    /// Types text at the cursor (`/` makes a fraction, `^` a superscript...).
    #[wasm_bindgen(js_name = typeText)]
    pub fn type_text(&mut self, text: &str) {
        self.editor.type_text(text);
    }

    /// Handles a key by its DOM name (`ArrowLeft`, `Backspace`...); false
    /// when the editor does not use it.
    pub fn key(&mut self, name: &str, shift: bool, command: bool) -> bool {
        match mathcore::Key::from_name(name, shift, command) {
            Some(k) => {
                self.editor.key(k);
                true
            }
            None => false,
        }
    }

    /// Inserts TeX at the cursor as structure (for paste).
    #[wasm_bindgen(js_name = insertTex)]
    pub fn insert_tex(&mut self, tex: &str) {
        self.editor.insert_tex(tex);
    }

    /// Runs a command by name: `frac`, `sqrt`, `nthroot`, `alpha`...
    pub fn command(&mut self, name: &str) {
        self.editor.command(name);
    }

    /// The selection as TeX, for copying ("" without one).
    #[wasm_bindgen(js_name = selectedTex)]
    pub fn selected_tex(&self) -> String {
        if self.editor.has_selection() {
            self.editor.selected_tex()
        } else {
            String::new()
        }
    }

    /// Lays the editor out and returns it as SVG; `caret()`, `selection()`
    /// and `metrics()` then describe this layout.
    #[wasm_bindgen(js_name = renderSvg)]
    pub fn render_svg(&mut self, engine: &MathEngine, font_size: f32, display_mode: bool, argb: u32) -> Result<String, JsError> {
        let opts = RenderOptions {
            font_size,
            display_mode,
            color: color_from_argb(argb),
            ..RenderOptions::default()
        };
        let l = self.editor.layout(&engine.font, &opts).map_err(|e| JsError::new(&e.to_string()))?;
        let svg = mathraster::to_svg(&engine.font, &l.display, 0.0);
        self.last = Some(l);
        Ok(svg)
    }

    /// `[x, y, width, height]` of the caret in the last layout.
    pub fn caret(&self) -> Vec<f32> {
        self.last
            .as_ref()
            .map_or(vec![0.0; 4], |l| vec![l.caret.x, l.caret.y, l.caret.width, l.caret.height])
    }

    /// Selection rectangles of the last layout, `[x, y, w, h] * n`.
    pub fn selection(&self) -> Vec<f32> {
        self.last
            .as_ref()
            .map_or(vec![], |l| l.selection.iter().flat_map(|r| [r.x, r.y, r.width, r.height]).collect())
    }

    /// `[width, ascent, descent]` of the last layout.
    pub fn metrics(&self) -> Vec<f32> {
        self.last
            .as_ref()
            .map_or(vec![0.0; 3], |l| vec![l.display.width, l.display.ascent, l.display.descent])
    }

    /// Moves the cursor to a tap at (x, y) in the last layout.
    pub fn tap(&mut self, x: f32, y: f32) {
        if let Some(l) = &self.last {
            self.editor.tap(l, x, y);
        }
    }

    /// What a screen reader says for the cursor's place, in `language`.
    pub fn describe(&self, language: Option<String>) -> String {
        self.editor.describe(&speech_opts(1, language))
    }

    /// The whole formula read aloud, in `language`.
    pub fn speech(&self, language: Option<String>) -> String {
        self.editor.speech(&speech_opts(1, language))
    }
}
