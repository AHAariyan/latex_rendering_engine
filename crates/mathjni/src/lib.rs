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

use jni::objects::{JByteArray, JByteBuffer, JClass, JString};
use jni::sys::{jboolean, jfloat, jfloatArray, jint, jlong, jstring};
use jni::JNIEnv;
use mathcore::{Color, LineBreak, Macros, MathFont, RenderOptions};
use std::cell::RefCell;
use std::sync::Mutex;
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
    /// Recomposing widgets ask for the same formula again and again.
    cache: mathcore::LayoutCache,
    budget: Mutex<mathcore::Budget>,
}

impl Engine {
    fn new(data: Vec<*mut [u8]>, font: MathFont<'static>) -> Self {
        Engine {
            data,
            font,
            cache: mathcore::LayoutCache::default(),
            budget: Mutex::new(mathcore::Budget::default()),
        }
    }
}

/// A panic inside the engine must not unwind into the JVM: it becomes an
/// error return. (Effective in builds with `panic = "unwind"`, which the SDK
/// profile uses.)
fn guard<T>(fallback: T, f: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(v) => v,
        Err(_) => {
            set_error("internal error in the math engine; please report the formula");
            fallback
        }
    }
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
    Ok(Box::new(Engine::new(owned, font)))
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
            Ok(font) => Box::into_raw(Box::new(Engine::new(Vec::new(), font))) as jlong,
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
    guard(std::ptr::null_mut(), || {
        render_impl(&mut env, handle, tex, font_size, display, color, macros, max_width, hit_testing)
    })
}

#[allow(clippy::too_many_arguments)]
fn render_impl(
    env: &mut JNIEnv,
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
        budget: *eng.budget.lock().unwrap_or_else(|p| p.into_inner()),
    };
    match eng.cache.render(&eng.font, &tex, &opts) {
        Ok(dl) => float_array(env, &pack(&dl)),
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

fn get(env: &mut JNIEnv, s: &JString) -> Option<String> {
    match env.get_string(s) {
        Ok(s) => Some(s.into()),
        Err(_) => {
            set_error("not a string");
            None
        }
    }
}

fn options(env: &mut JNIEnv, v: jint, language: &JString) -> mathcore::SpeechOptions {
    let tag: String = if language.is_null() {
        String::new()
    } else {
        env.get_string(language).map(Into::into).unwrap_or_default()
    };
    mathcore::SpeechOptions {
        verbosity: match v {
            0 => mathcore::Verbosity::Verbose,
            2 => mathcore::Verbosity::Superbrief,
            _ => mathcore::Verbosity::Brief,
        },
        language: mathcore::Language::from_tag(&tag),
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_speechWith(
    mut env: JNIEnv,
    _class: JClass,
    tex: JString,
    level: jint,
    language: JString,
) -> jstring {
    guard(std::ptr::null_mut(), || {
        let Some(tex) = get(&mut env, &tex) else {
            return std::ptr::null_mut();
        };
        let opts = options(&mut env, level, &language);
        string_result(&env, mathcore::render_speech_with(&tex, &Macros::new(), &opts))
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_speechTree(
    mut env: JNIEnv,
    _class: JClass,
    tex: JString,
    level: jint,
    language: JString,
) -> jstring {
    guard(std::ptr::null_mut(), || {
        let Some(tex) = get(&mut env, &tex) else {
            return std::ptr::null_mut();
        };
        let opts = options(&mut env, level, &language);
        string_result(&env, mathcore::render_speech_tree(&tex, &Macros::new(), &opts).map(|t| t.to_json()))
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_nemeth(mut env: JNIEnv, _class: JClass, tex: JString) -> jstring {
    guard(std::ptr::null_mut(), || {
        let Some(tex) = get(&mut env, &tex) else {
            return std::ptr::null_mut();
        };
        string_result(&env, mathcore::render_nemeth(&tex, &Macros::new()))
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_asciimathToTex(mut env: JNIEnv, _class: JClass, src: JString) -> jstring {
    guard(std::ptr::null_mut(), || {
        let Some(src) = get(&mut env, &src) else {
            return std::ptr::null_mut();
        };
        string_result(&env, mathcore::asciimath_to_tex(&src))
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_setBudget(
    _env: JNIEnv,
    _class: JClass,
    handle: jlong,
    max_expanded_bytes: jlong,
    max_nodes: jlong,
    max_items: jlong,
) {
    let Some(eng) = engine(handle) else { return };
    let d = mathcore::Budget::default();
    let pick = |v: jlong, default: usize| if v <= 0 { default } else { v as usize };
    *eng.budget.lock().unwrap_or_else(|p| p.into_inner()) = mathcore::Budget {
        max_expanded_bytes: pick(max_expanded_bytes, d.max_expanded_bytes),
        max_nodes: pick(max_nodes, d.max_nodes),
        max_items: pick(max_items, d.max_items),
    };
    eng.cache.clear();
}

/// Adds a text font (Bengali, Arabic, CJK...) to the end of the chain for
/// characters no earlier font has; `index` picks the face in a `.ttc`.
/// Returns the font's index in the chain, or -1 (see `lastError`). The
/// caller must not render on this engine from another thread meanwhile.
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_addFont(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    data: JByteArray,
    index: jint,
) -> jint {
    guard(-1, || {
        if handle == 0 || data.is_null() {
            set_error("null argument");
            return -1;
        }
        let Ok(bytes) = env.convert_byte_array(&data) else {
            set_error("cannot read font bytes");
            return -1;
        };
        let buf: *mut [u8] = Box::into_raw(bytes.into_boxed_slice());
        // SAFETY: handles come from Box::into_raw; the buffer is owned by the
        // engine from here and freed after its fonts.
        let eng = unsafe { &mut *(handle as *mut Engine) };
        match MathFont::from_text_bytes(unsafe { &*buf }, index.max(0) as u32) {
            Ok(f) => {
                eng.data.push(buf);
                add_font(eng, f)
            }
            Err(e) => {
                unsafe { drop(Box::from_raw(buf)) };
                set_error(e.to_string());
                -1
            }
        }
    })
}

/// `addFont` without a copy, for a font file the caller has memory-mapped
/// (`FileChannel.map`). The direct buffer must stay reachable, unchanged,
/// for as long as the engine lives.
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_addFontBuffer(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    buffer: JByteBuffer,
    index: jint,
) -> jint {
    guard(-1, || {
        if handle == 0 || buffer.is_null() {
            set_error("null argument");
            return -1;
        }
        let (Ok(ptr), Ok(len)) = (env.get_direct_buffer_address(&buffer), env.get_direct_buffer_capacity(&buffer)) else {
            set_error("not a direct buffer");
            return -1;
        };
        // SAFETY: the caller keeps the mapped buffer alive with the engine.
        let bytes: &'static [u8] = unsafe { std::slice::from_raw_parts(ptr, len) };
        let eng = unsafe { &mut *(handle as *mut Engine) };
        match MathFont::from_text_bytes(bytes, index.max(0) as u32) {
            Ok(f) => add_font(eng, f),
            Err(e) => {
                set_error(e.to_string());
                -1
            }
        }
    })
}

fn add_font(eng: &mut Engine, font: MathFont<'static>) -> jint {
    let i = eng.font.add_fallback(font);
    eng.cache.clear();
    i as jint
}

/// The languages spoken math is available in, comma-separated BCP 47 tags.
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_speechLanguages(env: JNIEnv, _class: JClass) -> jstring {
    let tags: Vec<&str> = mathcore::Language::ALL.iter().map(|l| l.tag()).collect();
    env.new_string(tags.join(",")).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// The characters of `tex` no font in the chain can draw ("" when covered).
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_missingChars(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    tex: JString,
    display: jboolean,
) -> jstring {
    guard(std::ptr::null_mut(), || {
        let Some(eng) = engine(handle) else {
            return std::ptr::null_mut();
        };
        let Some(tex) = get(&mut env, &tex) else {
            return std::ptr::null_mut();
        };
        let opts = RenderOptions {
            display_mode: display != 0,
            budget: *eng.budget.lock().unwrap_or_else(|p| p.into_inner()),
            ..RenderOptions::default()
        };
        string_result(
            &env,
            mathcore::missing_chars(&eng.font, &tex, &opts).map(|c| c.into_iter().collect()),
        )
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_setCacheCapacity(_env: JNIEnv, _class: JClass, handle: jlong, capacity: jint) {
    if let Some(eng) = engine(handle) {
        eng.cache.set_capacity(capacity.max(0) as usize);
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

// ---- The editor ----

pub struct EditorHandle {
    editor: mathcore::Editor,
    last: Option<mathcore::EditorLayout>,
}

fn editor<'a>(handle: jlong) -> Option<&'a mut EditorHandle> {
    if handle == 0 {
        set_error("editor handle is null");
        return None;
    }
    // SAFETY: handles are only produced by editorCreate and freed once; the
    // Kotlin side serializes calls on one editor.
    Some(unsafe { &mut *(handle as *mut EditorHandle) })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorCreate(_env: JNIEnv, _class: JClass) -> jlong {
    Box::into_raw(Box::new(EditorHandle {
        editor: mathcore::Editor::new(),
        last: None,
    })) as jlong
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorDestroy(_env: JNIEnv, _class: JClass, handle: jlong) {
    if handle != 0 {
        // SAFETY: see `editor`.
        unsafe { drop(Box::from_raw(handle as *mut EditorHandle)) };
    }
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorSetTex(mut env: JNIEnv, _class: JClass, handle: jlong, tex: JString) {
    guard((), || {
        if let (Some(e), Some(tex)) = (editor(handle), get(&mut env, &tex)) {
            e.editor = mathcore::Editor::from_tex(&tex);
            e.last = None;
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorTex(env: JNIEnv, _class: JClass, handle: jlong) -> jstring {
    guard(std::ptr::null_mut(), || match editor(handle) {
        Some(e) => string_result(&env, Ok(e.editor.tex())),
        None => std::ptr::null_mut(),
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorSelectedTex(env: JNIEnv, _class: JClass, handle: jlong) -> jstring {
    guard(std::ptr::null_mut(), || match editor(handle) {
        Some(e) => string_result(
            &env,
            Ok(if e.editor.has_selection() {
                e.editor.selected_tex()
            } else {
                String::new()
            }),
        ),
        None => std::ptr::null_mut(),
    })
}

/// `kind`: 0 types text, 1 inserts TeX as structure, 2 runs a command.
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorInput(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    kind: jint,
    text: JString,
) {
    guard((), || {
        if let (Some(e), Some(text)) = (editor(handle), get(&mut env, &text)) {
            match kind {
                0 => e.editor.type_text(&text),
                1 => e.editor.insert_tex(&text),
                _ => e.editor.command(&text),
            }
        }
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorKey(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    name: JString,
    shift: jboolean,
    command: jboolean,
) -> jboolean {
    guard(0, || {
        let (Some(e), Some(name)) = (editor(handle), get(&mut env, &name)) else {
            return 0;
        };
        match mathcore::Key::from_name(&name, shift != 0, command != 0) {
            Some(k) => {
                e.editor.key(k);
                1
            }
            None => 0,
        }
    })
}

/// Lays the editor out; the same packed floats as `render`.
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorRender(
    env: JNIEnv,
    _class: JClass,
    handle: jlong,
    engine_handle: jlong,
    font_size: jfloat,
    display: jboolean,
    argb: jint,
) -> jfloatArray {
    guard(std::ptr::null_mut(), || {
        let (Some(e), Some(eng)) = (editor(handle), engine(engine_handle)) else {
            return std::ptr::null_mut();
        };
        let opts = RenderOptions {
            font_size,
            display_mode: display != 0,
            color: color_from_argb(argb),
            ..RenderOptions::default()
        };
        match e.editor.layout(&eng.font, &opts) {
            Ok(l) => {
                let arr = float_array(&env, &pack(&l.display));
                e.last = Some(l);
                arr
            }
            Err(err) => {
                set_error(err.to_string());
                std::ptr::null_mut()
            }
        }
    })
}

/// The caret (x, y, width, height) followed by the selection rectangles
/// (four floats each) of the last layout.
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorCaret(env: JNIEnv, _class: JClass, handle: jlong) -> jfloatArray {
    guard(std::ptr::null_mut(), || {
        let Some(e) = editor(handle) else { return std::ptr::null_mut() };
        let mut flat = vec![0.0; 4];
        if let Some(l) = &e.last {
            flat = vec![l.caret.x, l.caret.y, l.caret.width, l.caret.height];
            flat.extend(l.selection.iter().flat_map(|r| [r.x, r.y, r.width, r.height]));
        }
        float_array(&env, &flat)
    })
}

#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorTap(_env: JNIEnv, _class: JClass, handle: jlong, x: jfloat, y: jfloat) {
    guard((), || {
        if let Some(e) = editor(handle) {
            if let Some(l) = &e.last {
                e.editor.tap(l, x, y);
            }
        }
    })
}

/// `whole` false: the cursor's place ("denominator, 2"); true: the formula.
#[no_mangle]
pub extern "system" fn Java_dev_mathcore_NativeBridge_editorSpeech(
    mut env: JNIEnv,
    _class: JClass,
    handle: jlong,
    whole: jboolean,
    language: JString,
) -> jstring {
    guard(std::ptr::null_mut(), || {
        let Some(e) = editor(handle) else { return std::ptr::null_mut() };
        let tag = if language.is_null() {
            String::new()
        } else {
            get(&mut env, &language).unwrap_or_default()
        };
        let opts = mathcore::SpeechOptions {
            verbosity: mathcore::Verbosity::Brief,
            language: mathcore::Language::from_tag(&tag),
        };
        string_result(
            &env,
            Ok(if whole != 0 {
                e.editor.speech(&opts)
            } else {
                e.editor.describe(&opts)
            }),
        )
    })
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
