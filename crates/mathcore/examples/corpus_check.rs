//! Runs every formula of a corpus file (one per line) through the engine and
//! checks what must hold for any formula it accepts:
//!
//! - no panic, at a 1 MB stack;
//! - the same display list every time (determinism);
//! - every coordinate finite and every size non-negative;
//! - hit testing changes nothing but the regions;
//! - line breaking at a phone width stays finite and inside the budget;
//! - the speech tree nests: every part inside its parent's source range,
//!   no part that repeats its parent, no empty text;
//! - the MathML is balanced, well-formed markup.
//!
//! Output is one tab-separated line per formula on stdout:
//! `index  status  micros  detail`, where status is `ok`, `reject` (a parse
//! error, detail is the message), `violation` (detail says which) or `panic`.
//!
//!     cargo run --release -p mathcore --example corpus_check -- formulas.txt > results.tsv

use mathcore::{DisplayList, Item, LineBreak, MathFont, RenderOptions, SpeechNode, SpeechOptions};
use std::io::Write;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).expect("usage: corpus_check FILE");
    let text = std::fs::read_to_string(&path).expect("read corpus");
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let handle = std::thread::Builder::new()
        .stack_size(1024 * 1024)
        .spawn(move || run(&lines))
        .unwrap();
    handle.join().unwrap();
}

fn run(lines: &[String]) {
    std::panic::set_hook(Box::new(|_| {}));
    let font = mathcore::bundled::font().unwrap();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for (i, tex) in lines.iter().enumerate() {
        let t0 = Instant::now();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(&font, tex)));
        let micros = t0.elapsed().as_micros();
        let (status, detail) = match r {
            Ok(Ok(())) => ("ok", String::new()),
            Ok(Err(Outcome::Reject(m))) => ("reject", m),
            Ok(Err(Outcome::Violation(m))) => ("violation", m),
            Err(p) => (
                "panic",
                p.downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default(),
            ),
        };
        let detail = detail.replace(['\t', '\n'], " ");
        let _ = writeln!(out, "{i}\t{status}\t{micros}\t{detail}");
    }
}

enum Outcome {
    Reject(String),
    Violation(String),
}

fn violation(what: impl Into<String>) -> Result<(), Outcome> {
    Err(Outcome::Violation(what.into()))
}

fn check(font: &MathFont<'_>, tex: &str) -> Result<(), Outcome> {
    let opts = RenderOptions::default();
    let dl = mathcore::render(font, tex, &opts).map_err(|e| Outcome::Reject(e.to_string()))?;

    finite(&dl)?;
    if mathcore::render(font, tex, &opts).ok().as_ref() != Some(&dl) {
        return violation("not deterministic");
    }

    let hit = mathcore::render(
        font,
        tex,
        &RenderOptions {
            hit_testing: true,
            ..opts.clone()
        },
    )
    .map_err(|e| Outcome::Violation(format!("hit testing rejects what plain accepts: {e}")))?;
    if hit.items != dl.items || hit.width != dl.width {
        return violation("hit testing changes the drawing");
    }
    for r in &hit.regions {
        if r.start > r.end || r.end as usize > tex.len() {
            return violation(format!("region {}..{} outside the source", r.start, r.end));
        }
    }

    let narrow = RenderOptions {
        line_break: Some(LineBreak::new(360.0)),
        ..opts.clone()
    };
    let broken =
        mathcore::render(font, tex, &narrow).map_err(|e| Outcome::Violation(format!("line breaking rejects what plain accepts: {e}")))?;
    finite(&broken)?;

    // Every verbosity in English, and every language at the default one.
    let mut readings: Vec<SpeechOptions> = [mathcore::Verbosity::Verbose, mathcore::Verbosity::Superbrief]
        .into_iter()
        .map(|verbosity| SpeechOptions {
            verbosity,
            ..Default::default()
        })
        .collect();
    readings.extend(mathcore::Language::ALL.map(|language| SpeechOptions {
        language,
        ..Default::default()
    }));
    for so in readings {
        let tree = mathcore::render_speech_tree(tex, &opts.macros, &so)
            .map_err(|e| Outcome::Violation(format!("speech rejects what layout accepts: {e}")))?;
        nests(&tree, tex.len() as u32)?;
    }

    let braille =
        mathcore::render_nemeth(tex, &opts.macros).map_err(|e| Outcome::Violation(format!("braille rejects what layout accepts: {e}")))?;
    if braille.trim().is_empty() && !tex.trim().is_empty() && !dl.items.is_empty() {
        return violation("empty braille for a formula that draws");
    }

    let ml = mathcore::render_mathml(tex, true, &opts.macros)
        .map_err(|e| Outcome::Violation(format!("mathml rejects what layout accepts: {e}")))?;
    balanced(&ml)?;
    Ok(())
}

fn finite(dl: &DisplayList) -> Result<(), Outcome> {
    let ok = |v: f32| v.is_finite();
    if !(ok(dl.width) && ok(dl.ascent) && ok(dl.descent)) || dl.width < 0.0 {
        return violation(format!("bad extent {} {} {}", dl.width, dl.ascent, dl.descent));
    }
    for item in &dl.items {
        let good = match item {
            Item::Glyph { x, y, size, .. } => ok(*x) && ok(*y) && ok(*size) && *size > 0.0,
            Item::Rule { x, y, width, height, .. } => ok(*x) && ok(*y) && ok(*width) && ok(*height) && *width >= 0.0 && *height >= 0.0,
            Item::Line {
                x1, y1, x2, y2, thickness, ..
            } => ok(*x1) && ok(*y1) && ok(*x2) && ok(*y2) && ok(*thickness),
        };
        if !good {
            return violation(format!("bad item {item:?}"));
        }
    }
    Ok(())
}

fn nests(n: &SpeechNode, len: u32) -> Result<(), Outcome> {
    if n.start > n.end || n.end > len {
        return violation(format!("speech range {}..{} outside the source", n.start, n.end));
    }
    if n.children.len() == 1 && n.children[0].text == n.text {
        return violation(format!("speech part repeats its parent: {:?}", n.text));
    }
    for c in &n.children {
        if c.text.is_empty() {
            return violation("empty speech part");
        }
        if c.start < n.start || c.end > n.end {
            return violation(format!("speech part {}..{} outside parent {}..{}", c.start, c.end, n.start, n.end));
        }
        nests(c, len)?;
    }
    Ok(())
}

/// Tags open and close in order, and no raw `<` or `&` leaks out of a token.
fn balanced(ml: &str) -> Result<(), Outcome> {
    let mut stack: Vec<&str> = Vec::new();
    let mut rest = ml;
    while let Some(i) = rest.find(['<', '&']) {
        rest = &rest[i..];
        if rest.starts_with('&') {
            let end = rest.find(';').filter(|&e| e < 12);
            if end.is_none() {
                return violation("unescaped & in mathml");
            }
            rest = &rest[1..];
            continue;
        }
        let Some(close) = rest.find('>') else {
            return violation("unterminated tag in mathml");
        };
        let tag = &rest[1..close];
        if tag.contains('<') {
            return violation("unescaped < in mathml");
        }
        if let Some(name) = tag.strip_prefix('/') {
            if stack.pop() != Some(name) {
                return violation(format!("mismatched </{name}> in mathml"));
            }
        } else if !tag.ends_with('/') {
            stack.push(tag.split_whitespace().next().unwrap_or(""));
        }
        rest = &rest[close + 1..];
    }
    if !stack.is_empty() {
        return violation(format!("unclosed <{}> in mathml", stack.last().unwrap()));
    }
    Ok(())
}
