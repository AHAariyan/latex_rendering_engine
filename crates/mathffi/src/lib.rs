//! C ABI for `mathcore`.
//!
//! The functions here are the single surface every platform binding is built
//! on. They deliberately expose only plain data: a flat item array and a
//! glyph-outline command stream. See `include/mathcore.h` for the contract.

use mathcore::{Color, Item, LineBreak, Macros, MathFont, RenderOptions};
use std::cell::RefCell;
use std::ffi::{c_char, CStr, CString};
use ttf_parser::OutlineBuilder;

thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Into<String>) {
    let msg: String = msg.into();
    LAST_ERROR.with(|e| *e.borrow_mut() = CString::new(msg).ok());
}

fn clear_error() {
    LAST_ERROR.with(|e| *e.borrow_mut() = None);
}

pub struct MathEngine {
    /// Leaked font bytes; reclaimed in `math_engine_free` after the font is
    /// dropped. `None` for the bundled fonts, which are static.
    data: Option<*mut [u8]>,
    font: MathFont<'static>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MathItem {
    pub kind: u8,
    /// Which font of the engine's chain a glyph belongs to; 0 is the primary.
    pub font: u16,
    pub glyph: u16,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub thickness: f32,
    pub color: u32,
}

/// Where a piece of the source ended up on screen. See `MathResult::regions`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MathRegion {
    pub start: u32,
    pub end: u32,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub depth: u16,
}

#[repr(C)]
pub struct MathResult {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub count: usize,
    pub items: *const MathItem,
    /// Empty unless `hit_testing` was asked for. Outermost first.
    pub region_count: usize,
    pub regions: *const MathRegion,
}

fn pack(c: Color) -> u32 {
    ((c.0 as u32) << 24) | ((c.1 as u32) << 16) | ((c.2 as u32) << 8) | c.3 as u32
}

fn unpack(c: u32) -> Color {
    Color((c >> 24) as u8, (c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// # Safety
/// `font_data` must point to `font_len` readable bytes.
#[no_mangle]
pub unsafe extern "C" fn math_engine_new(font_data: *const u8, font_len: usize) -> *mut MathEngine {
    clear_error();
    if font_data.is_null() {
        set_error("font_data is null");
        return std::ptr::null_mut();
    }
    let bytes = std::slice::from_raw_parts(font_data, font_len).to_vec().into_boxed_slice();
    let data: *mut [u8] = Box::into_raw(bytes);
    let font = match MathFont::from_bytes(&*data).map(|f| match MathFont::from_bytes(mathcore::bundled::FALLBACK) {
        Ok(fb) => f.with_fallback(fb),
        Err(_) => f,
    }) {
        Ok(f) => f,
        Err(e) => {
            drop(Box::from_raw(data));
            set_error(e.to_string());
            return std::ptr::null_mut();
        }
    };
    Box::into_raw(Box::new(MathEngine { data: Some(data), font }))
}

/// Creates an engine with the bundled Latin Modern Math font. Returns NULL when
/// the library was built without `bundled-font`.
#[no_mangle]
pub extern "C" fn math_engine_new_bundled() -> *mut MathEngine {
    clear_error();
    #[cfg(feature = "bundled-font")]
    {
        match mathcore::bundled::font() {
            Ok(font) => Box::into_raw(Box::new(MathEngine { data: None, font })),
            Err(e) => {
                set_error(e.to_string());
                std::ptr::null_mut()
            }
        }
    }
    #[cfg(not(feature = "bundled-font"))]
    {
        set_error("built without the bundled font");
        std::ptr::null_mut()
    }
}

/// # Safety
/// `engine` must come from `math_engine_new` or `math_engine_new_bundled` and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn math_engine_free(engine: *mut MathEngine) {
    if engine.is_null() {
        return;
    }
    let engine = Box::from_raw(engine);
    let data = engine.data;
    drop(engine);
    if let Some(data) = data {
        drop(Box::from_raw(data));
    }
}

/// # Safety
/// `engine` must be a live engine.
#[no_mangle]
pub unsafe extern "C" fn math_engine_units_per_em(engine: *const MathEngine, font: u16) -> f32 {
    if engine.is_null() {
        return 0.0;
    }
    let chain = &(*engine).font;
    if font as usize > chain.fallback_count() {
        return 0.0;
    }
    chain.font_at(font as usize).units_per_em()
}

/// # Safety
/// `engine` must be a live engine; `tex` must be a NUL-terminated UTF-8 string;
/// `macros` is either null or a NUL-terminated UTF-8 string. `max_width` of 0
/// or less renders one line of any width.
#[no_mangle]
pub unsafe extern "C" fn math_engine_render(
    engine: *const MathEngine,
    tex: *const c_char,
    font_size_px: f32,
    display_mode: bool,
    color: u32,
    macros: *const c_char,
    max_width: f32,
    hit_testing: bool,
) -> *mut MathResult {
    clear_error();
    if engine.is_null() || tex.is_null() {
        set_error("null argument");
        return std::ptr::null_mut();
    }
    let tex = match CStr::from_ptr(tex).to_str() {
        Ok(s) => s,
        Err(_) => {
            set_error("tex is not valid UTF-8");
            return std::ptr::null_mut();
        }
    };
    let defs = macros_from_c(macros);
    let opts = RenderOptions {
        font_size: font_size_px,
        display_mode,
        color: unpack(color),
        macros: defs,
        line_break: (max_width > 0.0).then(|| LineBreak::new(max_width)),
        hit_testing,
        budget: mathcore::Budget::default(),
    };
    let dl = match mathcore::render(&(*engine).font, tex, &opts) {
        Ok(dl) => dl,
        Err(e) => {
            set_error(e.to_string());
            return std::ptr::null_mut();
        }
    };
    let items: Vec<MathItem> = dl
        .items
        .iter()
        .map(|it| match *it {
            Item::Glyph {
                font,
                id,
                x,
                y,
                size,
                color,
            } => MathItem {
                kind: 0,
                font,
                glyph: id,
                x,
                y,
                w: size,
                h: 0.0,
                thickness: 0.0,
                color: pack(color),
            },
            Item::Rule {
                x,
                y,
                width,
                height,
                color,
            } => MathItem {
                kind: 1,
                font: 0,
                glyph: 0,
                x,
                y,
                w: width,
                h: height,
                thickness: 0.0,
                color: pack(color),
            },
            Item::Line {
                x1,
                y1,
                x2,
                y2,
                thickness,
                color,
            } => MathItem {
                kind: 2,
                font: 0,
                glyph: 0,
                x: x1,
                y: y1,
                w: x2,
                h: y2,
                thickness,
                color: pack(color),
            },
        })
        .collect();
    let count = items.len();
    let items = Box::leak(items.into_boxed_slice()).as_ptr();
    let regions: Vec<MathRegion> = dl
        .regions
        .iter()
        .map(|r| MathRegion {
            start: r.start,
            end: r.end,
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
            depth: r.depth,
        })
        .collect();
    let region_count = regions.len();
    let regions = Box::leak(regions.into_boxed_slice()).as_ptr();
    Box::into_raw(Box::new(MathResult {
        width: dl.width,
        ascent: dl.ascent,
        descent: dl.descent,
        count,
        items,
        region_count,
        regions,
    }))
}

/// # Safety
/// `result` must come from `math_engine_render` and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn math_result_free(result: *mut MathResult) {
    if result.is_null() {
        return;
    }
    let r = Box::from_raw(result);
    if !r.items.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(r.items as *mut MathItem, r.count)));
    }
    if !r.regions.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            r.regions as *mut MathRegion,
            r.region_count,
        )));
    }
}

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

/// Glyph outline in font units, y up, as the command stream documented in the
/// header. `font` is the index an item carries: 0 is the primary.
///
/// # Safety
/// `engine` must be a live engine; `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn math_engine_glyph_outline(engine: *const MathEngine, font: u16, glyph: u16, out_len: *mut usize) -> *mut f32 {
    if engine.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let chain = &(*engine).font;
    if font as usize > chain.fallback_count() {
        *out_len = 0;
        return std::ptr::null_mut();
    }
    let mut s = Stream(Vec::new());
    if !chain.font_at(font as usize).outline(ttf_parser::GlyphId(glyph), &mut s) || s.0.is_empty() {
        *out_len = 0;
        return std::ptr::null_mut();
    }
    *out_len = s.0.len();
    Box::leak(s.0.into_boxed_slice()).as_mut_ptr()
}

/// # Safety
/// `buffer`/`len` must come from `math_engine_glyph_outline`.
#[no_mangle]
pub unsafe extern "C" fn math_buffer_free(buffer: *mut f32, len: usize) {
    if !buffer.is_null() {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(buffer, len)));
    }
}

fn macros_from_c(macros: *const c_char) -> Macros {
    let mut defs = Macros::new();
    if !macros.is_null() {
        // SAFETY: the caller promises a NUL-terminated string or null.
        if let Ok(text) = unsafe { CStr::from_ptr(macros) }.to_str() {
            for line in text.lines() {
                if let Some((name, body)) = line.split_once('=') {
                    defs.define(name.trim(), body);
                }
            }
        }
    }
    defs
}

/// Presentation MathML for `tex`, for a screen reader. The caller owns the
/// string and must release it with `math_string_free`. NULL on a parse error.
///
/// # Safety
/// `tex` must be a NUL-terminated UTF-8 string; `macros` that or null.
#[no_mangle]
pub unsafe extern "C" fn math_mathml(tex: *const c_char, display_mode: bool, macros: *const c_char) -> *mut c_char {
    string_out(tex, macros, |t, m| mathcore::render_mathml(t, display_mode, m))
}

/// A spoken sentence for `tex`. Ownership and errors as `math_mathml`.
///
/// # Safety
/// `tex` must be a NUL-terminated UTF-8 string; `macros` that or null.
#[no_mangle]
pub unsafe extern "C" fn math_speech(tex: *const c_char, macros: *const c_char) -> *mut c_char {
    string_out(tex, macros, mathcore::render_speech)
}

unsafe fn string_out(tex: *const c_char, macros: *const c_char, f: impl Fn(&str, &Macros) -> mathcore::Result<String>) -> *mut c_char {
    clear_error();
    if tex.is_null() {
        set_error("tex is null");
        return std::ptr::null_mut();
    }
    let Ok(tex) = CStr::from_ptr(tex).to_str() else {
        set_error("tex is not valid UTF-8");
        return std::ptr::null_mut();
    };
    match f(tex, &macros_from_c(macros)) {
        Ok(s) => CString::new(s).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut()),
        Err(e) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

/// Releases a string returned by `math_mathml` or `math_speech`.
///
/// # Safety
/// `s` must come from one of those calls and not be used afterwards.
#[no_mangle]
pub unsafe extern "C" fn math_string_free(s: *mut c_char) {
    if !s.is_null() {
        drop(CString::from_raw(s));
    }
}

#[no_mangle]
pub extern "C" fn math_last_error() -> *const c_char {
    LAST_ERROR.with(|e| e.borrow().as_ref().map_or(std::ptr::null(), |s| s.as_ptr()))
}

#[no_mangle]
pub extern "C" fn math_version() -> *const c_char {
    static VERSION: &CStr = c"0.1.0";
    VERSION.as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math-subset.otf");

    #[test]
    fn round_trip_through_the_c_abi() {
        unsafe {
            let engine = math_engine_new(FONT.as_ptr(), FONT.len());
            assert!(!engine.is_null());
            assert_eq!(math_engine_units_per_em(engine, 0), 1000.0);
            let tex = CString::new(r"\half + \frac{a}{b}").unwrap();
            let macros = CString::new("\\half=\\frac{1}{2}").unwrap();
            let r = math_engine_render(engine, tex.as_ptr(), 32.0, true, 0xFF0000FF, macros.as_ptr(), 0.0, false);
            assert!(!r.is_null(), "{:?}", CStr::from_ptr(math_last_error()));
            let items = std::slice::from_raw_parts((*r).items, (*r).count);
            assert!(items.iter().filter(|i| i.kind == 1).count() == 2, "two fraction rules");
            assert!(items.iter().all(|i| i.color == 0xFF0000FF));
            assert!((*r).width > 0.0 && (*r).ascent > 0.0);
            let glyph = items.iter().find(|i| i.kind == 0).unwrap().glyph;
            let mut len = 0usize;
            let outline = math_engine_glyph_outline(engine, 0, glyph, &mut len);
            assert!(!outline.is_null() && len > 3);
            assert_eq!(*outline, 0.0, "starts with a move");
            math_buffer_free(outline, len);
            math_result_free(r);

            // Line breaking: the same formula gets taller and no wider than asked.
            let long = CString::new(r"a + b + c + d + e + f + g + h + i + j").unwrap();
            let wide = math_engine_render(engine, long.as_ptr(), 32.0, true, 0, std::ptr::null(), 0.0, false);
            let narrow = math_engine_render(engine, long.as_ptr(), 32.0, true, 0, std::ptr::null(), 150.0, false);
            assert!(!wide.is_null() && !narrow.is_null());
            assert!((*narrow).width <= 150.0 && (*narrow).width < (*wide).width);
            assert!((*narrow).ascent + (*narrow).descent > (*wide).ascent + (*wide).descent);
            math_result_free(wide);
            math_result_free(narrow);

            let bad = CString::new(r"\frac{a").unwrap();
            let r = math_engine_render(engine, bad.as_ptr(), 32.0, true, 0, std::ptr::null(), 0.0, false);
            assert!(r.is_null());
            let msg = CStr::from_ptr(math_last_error()).to_str().unwrap();
            assert!(msg.contains("parse error"), "{msg}");
            math_engine_free(engine);
        }
    }

    #[test]
    fn accessibility_strings_round_trip() {
        unsafe {
            let tex = CString::new(r"x^2 + \frac{1}{2}").unwrap();
            let ml = math_mathml(tex.as_ptr(), true, std::ptr::null());
            assert!(!ml.is_null());
            let s = CStr::from_ptr(ml).to_str().unwrap();
            assert!(s.starts_with("<math") && s.contains("<msup>") && s.contains("<mfrac>"));
            math_string_free(ml);

            let sp = math_speech(tex.as_ptr(), std::ptr::null());
            assert_eq!(CStr::from_ptr(sp).to_str().unwrap(), "x squared plus 1 over 2");
            math_string_free(sp);

            let bad = CString::new(r"\frac{a").unwrap();
            assert!(math_speech(bad.as_ptr(), std::ptr::null()).is_null());
            assert!(!math_last_error().is_null());
        }
    }

    #[test]
    fn hit_testing_maps_a_point_to_the_source() {
        unsafe {
            let engine = math_engine_new_bundled();
            // Hit testing: a tap on the first glyph names the source it came from.
            let hit_tex = CString::new(r"\frac{a}{b}+x").unwrap();
            let r = math_engine_render(engine, hit_tex.as_ptr(), 32.0, true, 0, std::ptr::null(), 0.0, true);
            assert!(!r.is_null());
            let regions = std::slice::from_raw_parts((*r).regions, (*r).region_count);
            assert!(!regions.is_empty());
            let items = std::slice::from_raw_parts((*r).items, (*r).count);
            let first = items.iter().find(|i| i.kind == 0).unwrap();
            let under: Vec<&MathRegion> = regions
                .iter()
                .filter(|g| {
                    first.x + 1.0 >= g.x && first.x + 1.0 <= g.x + g.width && first.y - 5.0 >= g.y && first.y - 5.0 <= g.y + g.height
                })
                .collect();
            assert!(!under.is_empty(), "no region under the first glyph");
            math_result_free(r);
            math_engine_free(engine);
        }
    }

    #[test]
    fn bundled_engine_works() {
        unsafe {
            let e = math_engine_new_bundled();
            assert!(!e.is_null());
            assert_eq!(math_engine_units_per_em(e, 0), 1000.0);
            math_engine_free(e);
        }
    }

    #[test]
    fn bad_font_reports_error() {
        unsafe {
            let e = math_engine_new(b"not a font".as_ptr(), 10);
            assert!(e.is_null());
            assert!(!math_last_error().is_null());
        }
    }
}
