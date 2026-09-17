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
use mathcore::{Color, LineBreak, Macros, MathFont, RenderOptions};
use std::cell::RefCell;
use ttf_parser::OutlineBuilder;

#[cfg(feature = "bundled-font")]
use mathcore::bundled;

thread_local! {
    static LAST_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

fn set_error(msg: impl Into<String>) {
    LAST_ERROR.with(|e| *e.borrow_mut() = Some(msg.into()));
}

pub struct Engine {
    /// Owned font bytes when loaded at runtime; empty for the bundled fonts.
    data: Vec<*mut [u8]>,
    font: MathFont<'static>,
}

impl Drop for Engine {
    fn drop(&mut self) {
        // SAFETY: every buffer came from Box::into_raw in `engine_from_bytes`,
        // and the fonts that borrow them are dropped with this struct's fields.
        for buf in std::mem::take(&mut self.data) {
            unsafe { drop(Box::from_raw(buf)) };
        }
    }
}

fn engine_from_bytes(math: Option<Vec<u8>>, text: Option<Vec<u8>>) -> Result<Box<Engine>, String> {
    let mut owned: Vec<*mut [u8]> = Vec::new();
    // SAFETY: each buffer stays alive until Engine::drop, which runs after the
    // fonts that borrow it.
    let mut load = |bytes: Vec<u8>| -> Result<MathFont<'static>, String> {
        let data: *mut [u8] = Box::into_raw(bytes.into_boxed_slice());
        match MathFont::from_bytes(unsafe { &*data }) {
            Ok(f) => {
                owned.push(data);
                Ok(f)
            }
            Err(e) => {
                unsafe { drop(Box::from_raw(data)) };
                Err(e.to_string())
            }
        }
    };
    let primary = match math {
        Some(b) => load(b)?,
        None => MathFont::from_bytes(bundled::PRIMARY).map_err(|e| e.to_string())?,
    };
    let mut font = match MathFont::from_bytes(bundled::FALLBACK) {
        Ok(fb) => primary.with_fallback(fb),
        Err(_) => primary,
    };
    if let Some(b) = text {
        font = font.with_text_font(load(b)?);
    }
    Ok(Box::new(Engine { data: owned, font }))
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
pub extern "system" fn Java_dev_mathcore_NativeBridge_create(
    env: JNIEnv,
    _class: JClass,
    font: JByteArray,
    text_font: JByteArray,
) -> jlong {
    let read = |a: &JByteArray| -> Option<Vec<u8>> {
        if a.is_null() {
            None
        } else {
            env.convert_byte_array(a).ok()
        }
    };
    let bytes = read(&font);
    if bytes.is_none() && !font.is_null() {
        set_error("cannot read font bytes");
        return 0;
    }
    match engine_from_bytes(bytes, read(&text_font)) {
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
        match bundled::font() {
            Ok(font) => Box::into_raw(Box::new(Engine { data: Vec::new(), font })) as jlong,
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
pub extern "system" fn Java_dev_mathcore_NativeBridge_unitsPerEm(_env: JNIEnv, _class: JClass, handle: jlong, font: jint) -> jfloat {
    engine(handle).map_or(0.0, |e| {
        if font as usize > e.font.fallback_count() {
            0.0
        } else {
            e.font.font_at(font as usize).units_per_em()
        }
    })
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
    max_width: jfloat,
    hit_testing: jboolean,
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
        line_break: (max_width > 0.0).then(|| LineBreak::new(max_width)),
        hit_testing: hit_testing != 0,
        budget: mathcore::Budget::default(),
    };
    match mathcore::render(&eng.font, &tex, &opts) {
        Ok(dl) => float_array(&env, &pack(&dl)),
        Err(e) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

fn string_result(env: &JNIEnv, r: mathcore::Result<String>) -> jstring {
    match r {
        Ok(s) => env.new_string(s).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut()),
        Err(e) => {
            set_error(e.to_string());
            std::ptr::null_mut()
        }
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_mathml(mut env: JNIEnv, _class: JClass, tex: JString, display: jboolean) -> jstring {
    let Ok(tex) = env.get_string(&tex) else {
        set_error("tex is not a string");
        return std::ptr::null_mut();
    };
    let tex: String = tex.into();
    string_result(&env, mathcore::render_mathml(&tex, display != 0, &Macros::new()))
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_speech(mut env: JNIEnv, _class: JClass, tex: JString) -> jstring {
    let Ok(tex) = env.get_string(&tex) else {
        set_error("tex is not a string");
        return std::ptr::null_mut();
    };
    let tex: String = tex.into();
    string_result(&env, mathcore::render_speech(&tex, &Macros::new()))
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
pub extern "system" fn Java_dev_mathcore_NativeBridge_glyphOutline(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    font: jint,
    glyph: jint,
) -> jfloatArray {
    let Some(eng) = engine(handle) else { return std::ptr::null_mut() };
    if font as usize > eng.font.fallback_count() {
        return std::ptr::null_mut();
    }
    let mut s = Stream(Vec::new());
    if !eng.font.font_at(font as usize).outline(ttf_parser::GlyphId(glyph as u16), &mut s) || s.0.is_empty() {
        return std::ptr::null_mut();
    }
    float_array(&env, &s.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_layout_matches_documentation() {
        let font = bundled::font().unwrap();
        let dl = mathcore::render(&font, r"\frac{a}{b}", &RenderOptions::default()).unwrap();
        let p = pack(&dl);
        let regions_at = 4 + dl.items.len() * 8;
        assert_eq!(p.len(), regions_at + 1, "items then an empty region block");
        assert_eq!(p[3] as usize, dl.items.len());
        assert_eq!(p[regions_at], 0.0, "no regions without hit testing");
        let kinds: Vec<f32> = p[4..regions_at].chunks(8).map(|c| c[0]).collect();
        assert!(kinds.contains(&1.0), "fraction rule present");
        assert_eq!(p[4 + 7].to_bits(), 0xFF000000, "opaque black in ARGB");

        // With hit testing the region block carries one record per atom.
        let opts = RenderOptions {
            hit_testing: true,
            ..Default::default()
        };
        let dl = mathcore::render(&font, r"\frac{a}{b}", &opts).unwrap();
        let p = pack(&dl);
        let regions_at = 4 + dl.items.len() * 8;
        assert_eq!(p[regions_at] as usize, dl.regions.len());
        assert_eq!(p.len(), regions_at + 1 + dl.regions.len() * 7);
    }

    #[test]
    fn runtime_font_engine_round_trips() {
        let e = engine_from_bytes(Some(bundled::PRIMARY.to_vec()), None).unwrap();
        assert_eq!(e.font.units_per_em(), 1000.0);
        assert!(engine_from_bytes(Some(b"junk".to_vec()), None).is_err());

        // A text font joins the chain after the bundled fallback.
        const LIB: &[u8] = include_bytes!("../../../assets/fonts/LibertinusMath-Regular.otf");
        let e = engine_from_bytes(None, Some(LIB.to_vec())).unwrap();
        assert_eq!(e.font.text_font(), Some(2));
    }
}
