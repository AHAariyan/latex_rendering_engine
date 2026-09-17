//! JNI bridge for Android.
//!
//! Every function here backs an `external fun` on `dev.mathcore.NativeBridge`.
//! Results cross the boundary as flat `FloatArray`s so no JNI object churn
//! happens per glyph. Layout of the render result:
//!
//! ```text
//! [width, ascent, descent, count,
//!  kind, glyph, x, y, w, h, thickness, colorBits,   <- item 0
//!  ...]
//! ```
//!
//! `kind` is 0 glyph, 1 rule, 2 line (same as the C ABI). `colorBits` is the
//! 0xAARRGGBB Android color reinterpreted as a float (`Float.fromBits`).

use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jfloat, jfloatArray, jint, jlong, jstring};
use jni::JNIEnv;
use mathcore::{Color, Macros, MathFont, RenderOptions};
use std::cell::RefCell;
use ttf_parser::OutlineBuilder;

#[cfg(feature = "bundled-font")]
const BUNDLED_FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math-subset.otf");

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Into<String>) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(msg.into()));
}

pub struct Engine {
    /// Owned font bytes when loaded at runtime; `None` for the bundled font.
    data: Option<*mut [u8]>,
    font: MathFont<'static>,
}

impl Drop for Engine {
    fn drop(&mut self) {
        if let Some(data) = self.data.take() {
            // The font borrows `data`; it is gone by the time this runs only if
            // the field order drops `font` first, so drop it explicitly.
            // SAFETY: `data` came from Box::into_raw in `engine_from_bytes`.
            unsafe { drop(Box::from_raw(data)) };
        }
    }
}

fn engine_from_bytes(bytes: Vec<u8>) -> Result<Box<Engine>, String> {
    let data: *mut [u8] = Box::into_raw(bytes.into_boxed_slice());
    // SAFETY: `data` stays alive until Engine::drop, which runs after `font` is dropped.
    let font = match MathFont::from_bytes(unsafe { &*data }) {
        Ok(f) => f,
        Err(e) => {
            unsafe { drop(Box::from_raw(data)) };
            return Err(e.to_string());
        }
    };
    Ok(Box::new(Engine { data: Some(data), font }))
}

/// Packs a display list into the flat float layout documented at the top.
pub fn pack(dl: &mathcore::DisplayList) -> Vec<f32> {
    dl.to_flat()
}

fn color_from_argb(argb: jint) -> Color {
    let v = argb as u32;
    Color((v >> 16) as u8, (v >> 8) as u8, v as u8, (v >> 24) as u8)
}

fn engine<'a>(handle: jlong) -> Option<&'a Engine> {
    if handle == 0 {
        set_error("engine handle is null");
        return None;
    }
    // SAFETY: handles are only ever produced by Box::into_raw below and freed once.
    Some(unsafe { &*(handle as *const Engine) })
}

fn float_array(env: &JNIEnv, data: &[f32]) -> jfloatArray {
    match env.new_float_array(data.len() as i32) {
        Ok(arr) => {
            if env.set_float_array_region(&arr, 0, data).is_err() {
                set_error("cannot fill float array");
                return std::ptr::null_mut();
            }
            arr.into_raw()
        }
        Err(_) => {
            set_error("cannot allocate float array");
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_create(env: JNIEnv, _class: JClass, font: JByteArray) -> jlong {
    let bytes = match env.convert_byte_array(&font) {
        Ok(b) => b,
        Err(_) => {
            set_error("cannot read font bytes");
            return 0;
        }
    };
    match engine_from_bytes(bytes) {
        Ok(e) => Box::into_raw(e) as jlong,
        Err(msg) => {
            set_error(msg);
            0
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_createBundled(_env: JNIEnv, _class: JClass) -> jlong {
    #[cfg(feature = "bundled-font")]
    {
        match MathFont::from_bytes(BUNDLED_FONT) {
            Ok(font) => Box::into_raw(Box::new(Engine { data: None, font })) as jlong,
            Err(e) => {
                set_error(e.to_string());
                0
            }
        }
    }
    #[cfg(not(feature = "bundled-font"))]
    {
        set_error("library built without the bundled font");
        0
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_destroy(_env: JNIEnv, _class: JClass, handle: jlong) {
    if handle != 0 {
        // SAFETY: see `engine`.
        unsafe { drop(Box::from_raw(handle as *mut Engine)) };
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_unitsPerEm(_env: JNIEnv, _class: JClass, handle: jlong) -> jfloat {
    engine(handle).map_or(0.0, |e| e.font.units_per_em())
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_render(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    tex: JString,
    font_size: jfloat,
    display: jboolean,
    color: jint,
    macros: JString,
) -> jfloatArray {
    let Some(eng) = engine(handle) else { return std::ptr::null_mut() };
    let tex: String = match env.get_string(&tex) {
        Ok(s) => s.into(),
        Err(_) => {
            set_error("tex is not a string");
            return std::ptr::null_mut();
        }
    };
    let mut defs = Macros::new();
    if !macros.is_null() {
        if let Ok(s) = env.get_string(&macros) {
            let s: String = s.into();
            for line in s.lines() {
                if let Some((name, body)) = line.split_once('=') {
                    defs.define(name.trim(), body);
                }
            }
        }
    }
    let opts = RenderOptions {
        font_size,
        display_mode: display != 0,
        color: color_from_argb(color),
        macros: defs,
    };
    match mathcore::render(&eng.font, &tex, &opts) {
        Ok(dl) => float_array(&env, &pack(&dl)),
        Err(e) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_lastError(env: JNIEnv, _class: JClass) -> jstring {
    let msg = LAST_ERROR.with(|e| e.borrow_mut().take());
    match msg {
        Some(m) => env.new_string(m).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut()),
        None => std::ptr::null_mut(),
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

/// Glyph outline in font units, y up, as the same command stream as the C ABI.
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_glyphOutline(env: JNIEnv, _class: JClass, handle: jlong, glyph: jint) -> jfloatArray {
    let Some(eng) = engine(handle) else { return std::ptr::null_mut() };
    let mut s = Stream(Vec::new());
    if !eng.font.outline(ttf_parser::GlyphId(glyph as u16), &mut s) || s.0.is_empty() {
        return std::ptr::null_mut();
    }
    float_array(&env, &s.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_layout_matches_documentation() {
        let font = MathFont::from_bytes(BUNDLED_FONT).unwrap();
        let dl = mathcore::render(&font, r"\frac{a}{b}", &RenderOptions::default()).unwrap();
        let p = pack(&dl);
        assert_eq!(p.len(), 4 + dl.items.len() * 8);
        assert_eq!(p[3] as usize, dl.items.len());
        let kinds: Vec<f32> = p[4..].chunks(8).map(|c| c[0]).collect();
        assert!(kinds.contains(&1.0), "fraction rule present");
        assert_eq!(p[4 + 7].to_bits(), 0xFF000000, "opaque black in ARGB");
    }

    #[test]
    fn runtime_font_engine_round_trips() {
        let e = engine_from_bytes(BUNDLED_FONT.to_vec()).unwrap();
        assert_eq!(e.font.units_per_em(), 1000.0);
        assert!(engine_from_bytes(b"junk".to_vec()).is_err());
    }
}
