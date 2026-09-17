//! Writes the reference display lists every binding must reproduce.
//!
//! `scripts/parity.sh` regenerates `tests/parity/expected.json` with this and
//! then has the WebAssembly and Dart bindings render the same corpus and
//! compare. The engine promises the same formula lays out identically on every
//! platform; this is what checks it. The bundled subset font is used because
//! that is what the bindings embed.

use mathcore::{Item, LineBreak, RenderOptions};

#[path = "../../mathraster/tests/common/corpus.rs"]
#[allow(dead_code)]
mod corpus;

fn json_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Three decimals is finer than a device pixel and immune to the last bit of
/// float noise between one target's code generation and another's.
fn num(v: f32) -> String {
    let r = (v * 1000.0).round() / 1000.0;
    if r == 0.0 {
        "0".into()
    } else {
        format!("{r}")
    }
}

fn argb(c: mathcore::Color) -> u32 {
    ((c.3 as u32) << 24) | ((c.0 as u32) << 16) | ((c.1 as u32) << 8) | c.2 as u32
}

fn main() {
    let font = mathcore::bundled::font().unwrap();
    let cases: Vec<(String, &str, bool, f32)> = corpus::CORPUS
        .iter()
        .map(|(n, t, d)| (n.to_string(), *t, *d, 0.0))
        .chain(corpus::LINEBREAK.iter().map(|(n, t, w)| (format!("lb_{n}"), *t, true, *w)))
        .collect();

    let mut out = String::from("{\n  \"fontSize\": 32,\n  \"cases\": [\n");
    for (i, (name, tex, display, width)) in cases.iter().enumerate() {
        let opts = RenderOptions {
            font_size: 32.0,
            display_mode: *display,
            line_break: (*width > 0.0).then(|| LineBreak::new(*width)),
            ..Default::default()
        };
        let dl = mathcore::render(&font, tex, &opts).unwrap_or_else(|e| panic!("{name}: {e}"));
        out.push_str("    {\"name\": ");
        json_string(name, &mut out);
        out.push_str(", \"tex\": ");
        json_string(tex, &mut out);
        out.push_str(&format!(
            ", \"display\": {display}, \"maxWidth\": {}, \"width\": {}, \"ascent\": {}, \"descent\": {}, \"items\": [",
            num(*width),
            num(dl.width),
            num(dl.ascent),
            num(dl.descent)
        ));
        for (j, item) in dl.items.iter().enumerate() {
            if j > 0 {
                out.push_str(", ");
            }
            let (k, g, a, b, c, d, t, col) = match *item {
                Item::Glyph {
                    font,
                    id,
                    x,
                    y,
                    size,
                    color,
                } => (0, id, x, y, size, font as f32, 0.0, color),
                Item::Rule {
                    x,
                    y,
                    width,
                    height,
                    color,
                } => (1, 0, x, y, width, height, 0.0, color),
                Item::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    thickness,
                    color,
                } => (2, 0, x1, y1, x2, y2, thickness, color),
            };
            out.push_str(&format!(
                "[{k},{g},{},{},{},{},{},{}]",
                num(a),
                num(b),
                num(c),
                num(d),
                num(t),
                argb(col)
            ));
        }
        out.push_str("]}");
        if i + 1 < cases.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str("  ]\n}\n");
    print!("{out}");
}
