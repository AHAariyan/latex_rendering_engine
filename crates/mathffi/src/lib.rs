//! C ABI for `mathcore`.
//!
//! The functions here are the single surface every platform binding is built
//! on. They deliberately expose only plain data: a flat item array and a
//! glyph-outline command stream. See `include/mathcore.h` for the contract.

use mathcore::{Color, Item, Macros, MathFont, RenderOptions};
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
    /// Leaked font bytes; reclaimed in `math_engine_free` after the font is dropped.
    /// `None` for the bundled font.
    data: Option<*mut [u8]>,
    font: MathFont<'static>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MathItem {
    pub kind: u8,
    pub glyph: u16,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub thickness: f32,
    pub color: u32,
}

#[repr(C)]
pub struct MathResult {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub count: usize,
    pub items: *const MathItem,
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
    let font = match MathFont::from_bytes(&*data) {
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
        const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");
        match MathFont::from_bytes(FONT) {
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
pub unsafe extern "C" fn math_engine_units_per_em(engine: *const MathEngine) -> f32 {
    if engine.is_null() {
        return 0.0;
    }
    (*engine).font.units_per_em()
}

/// # Safety
/// `engine` must be a live engine; `tex` must be a NUL-terminated UTF-8 string;
/// `macros` is either null or a NUL-terminated UTF-8 string.
#[no_mangle]
pub unsafe extern "C" fn math_engine_render(
    engine: *const MathEngine,
    tex: *const c_char,
    font_size_px: f32,
    display_mode: bool,
    color: u32,
    macros: *const c_char,
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
    let mut defs = Macros::new();
    if !macros.is_null() {
        if let Ok(text) = CStr::from_ptr(macros).to_str() {
            for line in text.lines() {
                if let Some((name, body)) = line.split_once('=') {
                    defs.define(name.trim(), body);
                }
            }
        }
    }
    let opts = RenderOptions {
        font_size: font_size_px,
        display_mode,
        color: unpack(color),
        macros: defs,
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
            Item::Glyph { id, x, y, size, color } => MathItem {
                kind: 0,
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
    Box::into_raw(Box::new(MathResult {
        width: dl.width,
        ascent: dl.ascent,
        descent: dl.descent,
        count,
        items,
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

/// # Safety
/// `engine` must be a live engine; `out_len` must be a valid pointer.
#[no_mangle]
pub unsafe extern "C" fn math_engine_glyph_outline(engine: *const MathEngine, glyph: u16, out_len: *mut usize) -> *mut f32 {
    if engine.is_null() || out_len.is_null() {
        return std::ptr::null_mut();
    }
    let mut s = Stream(Vec::new());
    if !(*engine).font.outline(ttf_parser::GlyphId(glyph), &mut s) || s.0.is_empty() {
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

    const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");

    #[test]
    fn round_trip_through_the_c_abi() {
        unsafe {
            let engine = math_engine_new(FONT.as_ptr(), FONT.len());
            assert!(!engine.is_null());
            assert_eq!(math_engine_units_per_em(engine), 1000.0);
            let tex = CString::new(r"\half + \frac{a}{b}").unwrap();
            let macros = CString::new("\\half=\\frac{1}{2}").unwrap();
            let r = math_engine_render(engine, tex.as_ptr(), 32.0, true, 0xFF0000FF, macros.as_ptr());
            assert!(!r.is_null(), "{:?}", CStr::from_ptr(math_last_error()));
            let items = std::slice::from_raw_parts((*r).items, (*r).count);
            assert!(items.iter().filter(|i| i.kind == 1).count() == 2, "two fraction rules");
            assert!(items.iter().all(|i| i.color == 0xFF0000FF));
            assert!((*r).width > 0.0 && (*r).ascent > 0.0);
            let glyph = items.iter().find(|i| i.kind == 0).unwrap().glyph;
            let mut len = 0usize;
            let outline = math_engine_glyph_outline(engine, glyph, &mut len);
            assert!(!outline.is_null() && len > 3);
            assert_eq!(*outline, 0.0, "starts with a move");
            math_buffer_free(outline, len);
            math_result_free(r);

            let bad = CString::new(r"\frac{a").unwrap();
            let r = math_engine_render(engine, bad.as_ptr(), 32.0, true, 0, std::ptr::null());
            assert!(r.is_null());
            let msg = CStr::from_ptr(math_last_error()).to_str().unwrap();
            assert!(msg.contains("parse error"), "{msg}");
            math_engine_free(engine);
        }
    }

    #[test]
    fn bundled_engine_works() {
        unsafe {
            let e = math_engine_new_bundled();
            assert!(!e.is_null());
            assert_eq!(math_engine_units_per_em(e), 1000.0);
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
