//! Accessibility output: MathML and spoken text.
//!
//! A formula drawn as glyphs is invisible to a screen reader. Both writers
//! walk the same `Node` tree the layout engine uses, so what a reader hears
//! is what the engine drew, and neither depends on a font.
//!
//! `mathml` produces Presentation MathML, which every major screen reader
//! consumes. `speech` produces a plain sentence for platforms that prefer a
//! label (Android `contentDescription`, iOS `accessibilityLabel`), following
//! the common conventions: "squared" for a square, "over" for a fraction,
//! "the square root of" for a radical.

use crate::ast::*;
use crate::symbols::styled_char;
use std::fmt::Write;

/// Presentation MathML for a parsed formula, without the `<math>` wrapper.
pub fn mathml(nodes: &[Node], display_mode: bool) -> String {
    let mut out = String::new();
    let _ = write!(
        out,
        r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="{}">"#,
        if display_mode { "block" } else { "inline" }
    );
    row(&mut out, nodes);
    out.push_str("</math>");
    out
}

/// A spoken rendering of a parsed formula.
pub fn speech(nodes: &[Node]) -> String {
    let mut out = String::new();
    say_list(&mut out, nodes);
    let mut text = out.split_whitespace().collect::<Vec<_>>().join(" ");
    text = text.replace(" ,", ",");
    text
}

// ---- MathML ---------------------------------------------------------------

fn escape(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
}

/// Wraps a list in `<mrow>` unless it is a single element already.
fn row(out: &mut String, nodes: &[Node]) {
    if nodes.len() == 1 {
        return node(out, &nodes[0]);
    }
    out.push_str("<mrow>");
    for n in nodes {
        node(out, n);
    }
    out.push_str("</mrow>");
}

fn one(out: &mut String, n: &Node) {
    match n {
        Node::Row(v) => row(out, v),
        _ => node(out, n),
    }
}

fn token(out: &mut String, tag: &str, text: &str) {
    let _ = write!(out, "<{tag}>");
    escape(out, text);
    let _ = write!(out, "</{tag}>");
}

fn node(out: &mut String, n: &Node) {
    match n {
        Node::Symbol { ch, atom, variant } => {
            let styled = styled_char(*ch, *variant);
            let tag = match atom {
                AtomType::Ord if styled.is_alphabetic() => "mi",
                AtomType::Ord if styled.is_numeric() => "mn",
                AtomType::Inner if styled.is_alphabetic() => "mi",
                _ => "mo",
            };
            if tag == "mi" && *variant == Variant::Roman {
                out.push_str(r#"<mi mathvariant="normal">"#);
                escape(out, &styled.to_string());
                out.push_str("</mi>");
            } else {
                token(out, tag, &styled.to_string());
            }
        }
        Node::Row(v) => row(out, v),
        Node::Scripts { base, sup, sub } => {
            let tag = match (sup, sub) {
                (Some(_), Some(_)) => "msubsup",
                (Some(_), None) => "msup",
                (None, Some(_)) => "msub",
                (None, None) => return one(out, base),
            };
            // A large operator takes its scripts above and below unless the
            // source said otherwise; MathML's own display logic then matches.
            let limits = match &**base {
                Node::BigOp { limits, .. } | Node::FnName { limits, .. } => *limits != Limits::NoLimits,
                Node::HBrace { .. } => true,
                _ => false,
            };
            let tag = if limits {
                match (sup, sub) {
                    (Some(_), Some(_)) => "munderover",
                    (Some(_), None) => "mover",
                    _ => "munder",
                }
            } else {
                tag
            };
            let _ = write!(out, "<{tag}>");
            one(out, base);
            if let Some(s) = sub {
                one(out, s);
            }
            if let Some(s) = sup {
                one(out, s);
            }
            let _ = write!(out, "</{tag}>");
        }
        Node::BigOp { ch, .. } => token(out, "mo", &ch.to_string()),
        Node::FnName { name, .. } => {
            token(out, "mi", name);
            out.push_str("<mo>&#x2061;</mo>"); // function application
        }
        Node::Frac {
            num, den, rule, delims, ..
        } => {
            if let Some((l, r)) = delims {
                out.push_str("<mrow>");
                if let Some(c) = l {
                    token(out, "mo", &c.to_string());
                }
                frac(out, num, den, *rule);
                if let Some(c) = r {
                    token(out, "mo", &c.to_string());
                }
                out.push_str("</mrow>");
            } else {
                frac(out, num, den, *rule);
            }
        }
        Node::Sqrt { radicand, index } => match index {
            Some(i) => {
                out.push_str("<mroot>");
                one(out, radicand);
                one(out, i);
                out.push_str("</mroot>");
            }
            None => {
                out.push_str("<msqrt>");
                one(out, radicand);
                out.push_str("</msqrt>");
            }
        },
        Node::LeftRight { left, body, right } => {
            out.push_str("<mrow>");
            if let Some(c) = left {
                let _ = write!(out, r#"<mo stretchy="true">"#);
                escape(out, &c.to_string());
                out.push_str("</mo>");
            }
            row(out, body);
            if let Some(c) = right {
                let _ = write!(out, r#"<mo stretchy="true">"#);
                escape(out, &c.to_string());
                out.push_str("</mo>");
            }
            out.push_str("</mrow>");
        }
        Node::Middle(ch) | Node::SizedDelim { ch, .. } => token(out, "mo", &ch.to_string()),
        Node::Accent { ch, base, under, .. } => {
            let tag = if *under { ("munder", "accentunder") } else { ("mover", "accent") };
            let _ = write!(out, r#"<{} {}="true">"#, tag.0, tag.1);
            one(out, base);
            token(out, "mo", &ch.to_string());
            let _ = write!(out, "</{}>", tag.0);
        }
        Node::Overline(inner) => {
            out.push_str(r#"<mover accent="true">"#);
            one(out, inner);
            out.push_str("<mo>&#x00AF;</mo></mover>");
        }
        Node::Underline(inner) => {
            out.push_str(r#"<munder accentunder="true">"#);
            one(out, inner);
            out.push_str("<mo>&#x005F;</mo></munder>");
        }
        Node::Style { style, body } => {
            let level = match style {
                MathStyle::Display => r#"<mstyle displaystyle="true" scriptlevel="0">"#,
                MathStyle::Text => r#"<mstyle displaystyle="false" scriptlevel="0">"#,
                MathStyle::Script => r#"<mstyle displaystyle="false" scriptlevel="1">"#,
                MathStyle::ScriptScript => r#"<mstyle displaystyle="false" scriptlevel="2">"#,
            };
            out.push_str(level);
            row(out, body);
            out.push_str("</mstyle>");
        }
        Node::Text { text, .. } => token(out, "mtext", text),
        Node::Space { mu } => {
            let _ = write!(out, r#"<mspace width="{:.3}em"/>"#, mu / 18.0);
        }
        Node::Array(a) => {
            out.push_str("<mtable>");
            for r in &a.rows {
                out.push_str("<mtr>");
                for cell in r {
                    out.push_str("<mtd>");
                    row(out, cell);
                    out.push_str("</mtd>");
                }
                out.push_str("</mtr>");
            }
            out.push_str("</mtable>");
        }
        Node::Phantom { body, .. } => {
            out.push_str("<mphantom>");
            one(out, body);
            out.push_str("</mphantom>");
        }
        Node::OverUnder { base, over, under } => {
            let tag = match (over, under) {
                (Some(_), Some(_)) => "munderover",
                (Some(_), None) => "mover",
                (None, Some(_)) => "munder",
                (None, None) => return one(out, base),
            };
            let _ = write!(out, "<{tag}>");
            one(out, base);
            if let Some(u) = under {
                one(out, u);
            }
            if let Some(o) = over {
                one(out, o);
            }
            let _ = write!(out, "</{tag}>");
        }
        Node::Color { color, body } => {
            let _ = write!(out, r##"<mstyle mathcolor="#{:02x}{:02x}{:02x}">"##, color.0, color.1, color.2);
            row(out, body);
            out.push_str("</mstyle>");
        }
        Node::Boxed(inner) => {
            out.push_str(r#"<menclose notation="box">"#);
            one(out, inner);
            out.push_str("</menclose>");
        }
        Node::Cancel { body, .. } => {
            out.push_str(r#"<menclose notation="updiagonalstrike">"#);
            one(out, body);
            out.push_str("</menclose>");
        }
        Node::HBrace { base, over } => {
            let (tag, ch) = if *over { ("mover", '⏞') } else { ("munder", '⏟') };
            let _ = write!(out, "<{tag}>");
            one(out, base);
            token(out, "mo", &ch.to_string());
            let _ = write!(out, "</{tag}>");
        }
        Node::XArrow { ch, over, under } => {
            let tag = match (over, under) {
                (Some(_), Some(_)) => "munderover",
                (None, Some(_)) => "munder",
                _ => "mover",
            };
            let _ = write!(out, "<{tag}>");
            let _ = write!(out, r#"<mo stretchy="true">"#);
            escape(out, &ch.to_string());
            out.push_str("</mo>");
            if let Some(u) = under {
                one(out, u);
            }
            if let Some(o) = over {
                one(out, o);
            }
            let _ = write!(out, "</{tag}>");
        }
        Node::Class { body, .. } => one(out, body),
        Node::Raise { body, .. } | Node::VCenter(body) => one(out, body),
        Node::ColorBox { body, .. } => {
            out.push_str(r#"<menclose notation="box">"#);
            one(out, body);
            out.push_str("</menclose>");
        }
        Node::Lap { body, .. } => {
            out.push_str("<mpadded width=\"0\">");
            one(out, body);
            out.push_str("</mpadded>");
        }
        Node::Choice(b) => one(out, &b[0]),
        Node::Rule { width, height, .. } => {
            let _ = write!(
                out,
                r#"<mspace width="{width}em" height="{height}em" mathbackground="currentColor"/>"#
            );
        }
        Node::Spanned { body, .. } => one(out, body),
    }
}

fn frac(out: &mut String, num: &Node, den: &Node, rule: FracRule) {
    match rule {
        FracRule::None => out.push_str(r#"<mfrac linethickness="0">"#),
        _ => out.push_str("<mfrac>"),
    }
    one(out, num);
    one(out, den);
    out.push_str("</mfrac>");
}

// ---- speech ---------------------------------------------------------------

fn say_list(out: &mut String, nodes: &[Node]) {
    for n in nodes {
        say(out, n);
    }
}

fn word(out: &mut String, w: &str) {
    if !out.is_empty() && !out.ends_with(' ') {
        out.push(' ');
    }
    out.push_str(w);
}

fn say_one(out: &mut String, n: &Node) {
    match n {
        Node::Row(v) => say_list(out, v),
        _ => say(out, n),
    }
}

/// Renders a node to a fresh string, to test how short it is.
fn said(n: &Node) -> String {
    let mut s = String::new();
    say_one(&mut s, n);
    s.trim().to_string()
}

/// True for something short enough to read without "the ... of" scaffolding.
fn simple(n: &Node) -> bool {
    let s = said(n);
    !s.is_empty() && s.split_whitespace().count() == 1
}

fn say(out: &mut String, n: &Node) {
    match n {
        Node::Symbol { ch, variant, .. } => word(out, &symbol_name(styled_char(*ch, *variant))),
        Node::Row(v) => say_list(out, v),
        Node::Scripts { base, sup, sub } => {
            let limits = matches!(**base, Node::BigOp { .. } | Node::FnName { .. } | Node::HBrace { .. });
            if limits {
                say_one(out, base);
                if let Some(s) = sub {
                    word(out, "from");
                    say_one(out, s);
                }
                if let Some(s) = sup {
                    word(out, "to");
                    say_one(out, s);
                }
                word(out, "of");
                return;
            }
            say_one(out, base);
            if let Some(s) = sub {
                word(out, "sub");
                say_one(out, s);
            }
            if let Some(s) = sup {
                match said(s).as_str() {
                    "2" => word(out, "squared"),
                    "3" => word(out, "cubed"),
                    _ if simple(s) => {
                        word(out, "to the");
                        say_one(out, s);
                    }
                    _ => {
                        word(out, "to the power of");
                        say_one(out, s);
                        word(out, ",");
                    }
                }
            }
        }
        Node::BigOp { ch, .. } => word(out, &big_op_name(*ch)),
        Node::FnName { name, .. } => word(out, name),
        Node::Frac {
            num, den, rule, delims, ..
        } => {
            if delims.is_some() && *rule == FracRule::None {
                say_one(out, num);
                word(out, "choose");
                say_one(out, den);
                return;
            }
            let short = simple(num) && simple(den);
            if !short {
                word(out, "the fraction");
            }
            say_one(out, num);
            word(out, "over");
            say_one(out, den);
            if !short {
                word(out, ",");
            }
        }
        Node::Sqrt { radicand, index } => {
            match index {
                Some(i) => {
                    word(out, "the");
                    say_one(out, i);
                    word(out, "th root of");
                }
                None => word(out, "the square root of"),
            }
            say_one(out, radicand);
            if !simple(radicand) {
                word(out, ",");
            }
        }
        Node::LeftRight { left, body, right } => {
            if let Some(c) = left {
                word(out, open_name(*c));
            }
            say_list(out, body);
            if let Some(c) = right {
                word(out, close_name(*c));
            }
        }
        Node::Middle(ch) | Node::SizedDelim { ch, .. } => word(out, &symbol_name(*ch)),
        Node::Accent { ch, base, .. } => {
            say_one(out, base);
            word(out, accent_name(*ch));
        }
        Node::Overline(inner) => {
            say_one(out, inner);
            word(out, "bar");
        }
        Node::Underline(inner) => {
            say_one(out, inner);
            word(out, "underlined");
        }
        Node::Style { body, .. } => say_list(out, body),
        Node::Color { body, .. } => say_list(out, body),
        Node::Text { text, .. } => word(out, text.trim()),
        Node::Space { .. } => {}
        Node::Array(a) => {
            let rows = a.rows.len();
            let cols = a.rows.iter().map(|r| r.len()).max().unwrap_or(0);
            if cols > 1 {
                word(out, &format!("the {rows} by {cols} matrix,"));
            } else {
                word(out, &format!("{rows} rows,"));
            }
            for (i, r) in a.rows.iter().enumerate() {
                word(out, &format!("row {},", i + 1));
                for (j, cell) in r.iter().enumerate() {
                    if j > 0 {
                        word(out, ",");
                    }
                    say_list(out, cell);
                }
                word(out, ",");
            }
        }
        Node::Phantom { .. } => {}
        Node::OverUnder { base, over, under } => {
            say_one(out, base);
            if let Some(u) = under {
                word(out, "under");
                say_one(out, u);
            }
            if let Some(o) = over {
                word(out, "over");
                say_one(out, o);
            }
        }
        Node::Boxed(inner) => {
            word(out, "boxed,");
            say_one(out, inner);
        }
        Node::Cancel { body, .. } => {
            word(out, "crossed out,");
            say_one(out, body);
        }
        Node::HBrace { base, over } => {
            word(out, if *over { "over brace of" } else { "under brace of" });
            say_one(out, base);
        }
        Node::XArrow { ch, over, under } => {
            word(out, &symbol_name(*ch));
            if let Some(o) = over {
                word(out, "with");
                say_one(out, o);
            }
            if let Some(u) = under {
                word(out, "under");
                say_one(out, u);
            }
        }
        Node::Class { body, .. } => say_one(out, body),
        Node::Raise { body, .. } | Node::VCenter(body) | Node::ColorBox { body, .. } | Node::Lap { body, .. } => say_one(out, body),
        Node::Choice(b) => say_one(out, &b[0]),
        Node::Rule { .. } => {}
        Node::Spanned { body, .. } => say_one(out, body),
    }
}

fn open_name(c: char) -> &'static str {
    match c {
        '(' => "open paren",
        '[' => "open bracket",
        '{' => "open brace",
        '⟨' => "open angle bracket",
        '|' => "the absolute value of",
        '‖' => "the norm of",
        '⌊' => "the floor of",
        '⌈' => "the ceiling of",
        _ => "open",
    }
}

fn close_name(c: char) -> &'static str {
    match c {
        '(' | ')' => "close paren",
        '[' | ']' => "close bracket",
        '{' | '}' => "close brace",
        '⟩' => "close angle bracket",
        '|' | '‖' | '⌋' | '⌉' => "",
        _ => "close",
    }
}

fn accent_name(c: char) -> &'static str {
    match c {
        '\u{0302}' => "hat",
        '\u{0303}' => "tilde",
        '\u{0304}' => "bar",
        '\u{20D7}' => "vector",
        '\u{0307}' => "dot",
        '\u{0308}' => "double dot",
        '\u{0301}' => "acute",
        '\u{0300}' => "grave",
        '\u{030C}' => "check",
        '\u{0306}' => "breve",
        '\u{030A}' => "ring",
        _ => "accent",
    }
}

fn big_op_name(c: char) -> String {
    match c {
        '∑' => "the sum",
        '∏' => "the product",
        '∐' => "the coproduct",
        '∫' => "the integral",
        '∬' => "the double integral",
        '∭' => "the triple integral",
        '∮' => "the contour integral",
        '⋃' => "the union",
        '⋂' => "the intersection",
        '⋁' => "the logical or",
        '⋀' => "the logical and",
        '⨁' => "the direct sum",
        '⨂' => "the tensor product",
        '⨄' => "the disjoint union",
        '⨆' => "the square union",
        _ => return symbol_name(c),
    }
    .to_string()
}

/// A spoken name for a character. Letters and digits are read as they are.
fn symbol_name(c: char) -> String {
    // Undo the math alphabets so "𝑥" reads as "x" and "𝜕" as "partial".
    if let Some(p) = plain_letter(c) {
        return p.to_string();
    }
    let c = plain_greek(c).unwrap_or(c);
    let name = match c {
        '+' => "plus",
        '−' | '-' => "minus",
        '±' => "plus or minus",
        '∓' => "minus or plus",
        '×' => "times",
        '÷' => "divided by",
        '⋅' | '·' => "dot",
        '∗' | '*' => "star",
        '=' => "equals",
        '≠' => "is not equal to",
        '<' => "is less than",
        '>' => "is greater than",
        '≤' => "is less than or equal to",
        '≥' => "is greater than or equal to",
        '≈' => "is approximately",
        '≡' => "is equivalent to",
        '∼' => "is similar to",
        '≅' => "is congruent to",
        '∝' => "is proportional to",
        '∈' => "is in",
        '∉' => "is not in",
        '∋' => "contains",
        '⊂' => "is a proper subset of",
        '⊆' => "is a subset of",
        '⊃' => "is a proper superset of",
        '⊇' => "is a superset of",
        '∪' => "union",
        '∩' => "intersection",
        '∖' => "set minus",
        '∅' => "the empty set",
        '∞' => "infinity",
        '∂' => "partial",
        '∇' => "del",
        'ℏ' => "h bar",
        '→' => "goes to",
        '←' => "from",
        '↔' => "if and only if",
        '⇒' => "implies",
        '⇐' => "is implied by",
        '⇔' => "if and only if",
        '↦' => "maps to",
        '∀' => "for all",
        '∃' => "there exists",
        '∄' => "there does not exist",
        '¬' => "not",
        '∧' => "and",
        '∨' => "or",
        '⊕' => "direct sum",
        '⊗' => "tensor",
        '∘' => "composed with",
        '|' => "bar",
        '‖' => "double bar",
        ',' => ",",
        ';' => ";",
        '.' => ".",
        ':' => "colon",
        '!' => "factorial",
        '′' => "prime",
        '″' => "double prime",
        '‴' => "triple prime",
        '…' | '⋯' => "and so on",
        '⋮' | '⋱' => "and so on",
        '(' => "open paren",
        ')' => "close paren",
        '[' => "open bracket",
        ']' => "close bracket",
        '{' => "open brace",
        '}' => "close brace",
        '°' => "degrees",
        '%' => "percent",
        '√' => "the square root of",
        _ => return greek_name(c).unwrap_or_else(|| c.to_string()),
    };
    name.to_string()
}

/// Maps a Mathematical Alphanumeric Symbol back to its ASCII letter or digit.
fn plain_letter(c: char) -> Option<char> {
    if c.is_ascii_alphanumeric() {
        return Some(c);
    }
    let v = c as u32;
    // Reserved holes that live elsewhere in Unicode.
    let hole = match c {
        'ℎ' => Some('h'),
        'ℬ' => Some('B'),
        'ℰ' => Some('E'),
        'ℱ' => Some('F'),
        'ℋ' => Some('H'),
        'ℐ' => Some('I'),
        'ℒ' => Some('L'),
        'ℳ' => Some('M'),
        'ℛ' => Some('R'),
        'ℯ' => Some('e'),
        'ℊ' => Some('g'),
        'ℴ' => Some('o'),
        'ℭ' => Some('C'),
        'ℌ' => Some('H'),
        'ℑ' => Some('I'),
        'ℜ' => Some('R'),
        'ℨ' => Some('Z'),
        'ℂ' => Some('C'),
        'ℍ' => Some('H'),
        'ℕ' => Some('N'),
        'ℙ' => Some('P'),
        'ℚ' => Some('Q'),
        'ℝ' => Some('R'),
        'ℤ' => Some('Z'),
        _ => None,
    };
    if hole.is_some() {
        return hole;
    }
    if (0x1D400..=0x1D7CB).contains(&v) {
        // Each alphabet is 26 upper, 26 lower; Greek and digit runs differ.
        let off = (v - 0x1D400) % 52;
        if v < 0x1D6A8 {
            return char::from_u32(if off < 26 { 'A' as u32 + off } else { 'a' as u32 + off - 26 });
        }
    }
    if (0x1D7CE..=0x1D7FF).contains(&v) {
        return char::from_u32('0' as u32 + (v - 0x1D7CE) % 10);
    }
    None
}

/// Maps a Mathematical Alphanumeric Greek letter back to its plain form.
fn plain_greek(c: char) -> Option<char> {
    let v = c as u32;
    // Bold, italic, bold italic, sans-serif bold, sans-serif bold italic.
    let base = [0x1D6A8u32, 0x1D6E2, 0x1D71C, 0x1D756, 0x1D790]
        .into_iter()
        .find(|b| (*b..b + 58).contains(&v))?;
    let idx = v - base;
    Some(match idx {
        17 => 'ϴ',
        0..=24 => char::from_u32(0x391 + idx)?,
        25 => '∇',
        26..=50 => char::from_u32(0x3B1 + idx - 26)?,
        51 => '∂',
        52 => 'ϵ',
        53 => 'ϑ',
        54 => 'ϰ',
        55 => 'ϕ',
        56 => 'ϱ',
        _ => 'ϖ',
    })
}

fn greek_name(c: char) -> Option<String> {
    let name = match c {
        'α' => "alpha",
        'β' => "beta",
        'γ' => "gamma",
        'δ' => "delta",
        'ε' | 'ϵ' => "epsilon",
        'ζ' => "zeta",
        'η' => "eta",
        'θ' | 'ϑ' => "theta",
        'ι' => "iota",
        'κ' => "kappa",
        'λ' => "lambda",
        'μ' => "mu",
        'ν' => "nu",
        'ξ' => "xi",
        'π' | 'ϖ' => "pi",
        'ρ' | 'ϱ' => "rho",
        'σ' | 'ς' => "sigma",
        'τ' => "tau",
        'υ' => "upsilon",
        'φ' | 'ϕ' => "phi",
        'χ' => "chi",
        'ψ' => "psi",
        'ω' => "omega",
        'Γ' => "capital gamma",
        'Δ' => "capital delta",
        'Θ' => "capital theta",
        'Λ' => "capital lambda",
        'Ξ' => "capital xi",
        'Π' => "capital pi",
        'Σ' => "capital sigma",
        'Υ' => "capital upsilon",
        'Φ' => "capital phi",
        'Ψ' => "capital psi",
        'Ω' => "capital omega",
        _ => return None,
    };
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn ml(tex: &str) -> String {
        mathml(&parse(tex).unwrap(), true)
    }

    fn sp(tex: &str) -> String {
        speech(&parse(tex).unwrap())
    }

    #[test]
    fn mathml_structure() {
        assert!(ml("x^2").contains("<msup><mi>𝑥</mi><mn>2</mn></msup>"));
        assert!(ml(r"\frac{a}{b}").contains("<mfrac><mi>𝑎</mi><mi>𝑏</mi></mfrac>"));
        assert!(ml(r"\sqrt{2}").contains("<msqrt><mn>2</mn></msqrt>"));
        assert!(ml(r"\sqrt[3]{x}").contains("<mroot>"));
        assert!(ml(r"\binom{n}{k}").contains(r#"<mfrac linethickness="0">"#));
        assert!(ml(r"\sum_{i=1}^{n}").contains("<munderover>"));
        assert!(ml(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}").contains("<mtable><mtr><mtd>"));
        assert!(ml(r"\left( x \right)").contains(r#"<mo stretchy="true">(</mo>"#));
        assert!(ml(r"\text{if } x").contains("<mtext>if </mtext>"));
        assert!(ml(r"\mathrm{d}").contains(r#"<mi mathvariant="normal">d</mi>"#));
    }

    #[test]
    fn mathml_escapes_markup() {
        let out = ml("a < b > c");
        assert!(out.contains("&lt;") && out.contains("&gt;"));
        assert!(!out.contains("<mo><</mo>"));
    }

    #[test]
    fn mathml_wraps_the_whole_formula() {
        let out = ml("x + y");
        assert!(out.starts_with(r#"<math xmlns="http://www.w3.org/1998/Math/MathML" display="block">"#));
        assert!(out.ends_with("</math>"));
        assert!(mathml(&parse("x").unwrap(), false).contains(r#"display="inline""#));
    }

    #[test]
    fn speech_reads_like_a_sentence() {
        assert_eq!(sp("x^2 + y^2 = z^2"), "x squared plus y squared equals z squared");
        assert_eq!(sp(r"\frac{1}{2}"), "1 over 2");
        assert_eq!(sp(r"\frac{a+b}{c}"), "the fraction a plus b over c,");
        assert_eq!(sp(r"\sqrt{2}"), "the square root of 2");
        assert_eq!(sp(r"\sum_{i=1}^{n} i"), "the sum from i equals 1 to n of i");
        assert_eq!(sp(r"\alpha \le \beta"), "alpha is less than or equal to beta");
        assert_eq!(sp(r"\hat{x}"), "x hat");
        assert_eq!(sp(r"\left| x \right|"), "the absolute value of x");
        assert_eq!(sp(r"f(x)"), "f open paren x close paren");
        assert_eq!(sp(r"x \in \mathbb{R}"), "x is in R");
        assert_eq!(sp(r"\binom{n}{k}"), "n choose k");
        assert_eq!(sp(r"\frac{\partial f}{\partial x}"), "the fraction partial f over partial x,");
        assert_eq!(sp(r"\nabla \cdot \mathbf{E}"), "del dot E");
    }

    #[test]
    fn speech_handles_structures() {
        assert!(sp(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}").contains("the 2 by 2 matrix"));
        assert!(sp(r"\text{if } x > 0").starts_with("if x is greater than 0"));
        assert!(sp(r"x^{n+1}").contains("to the power of"));
        assert!(!sp(r"\phantom{x} y").contains('x'));
    }

    #[test]
    fn every_corpus_formula_produces_output() {
        for tex in [
            r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
            r"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}",
            r"f(x) = \begin{cases} x^2 & \text{if } x \ge 0 \\ -x & \text{otherwise} \end{cases}",
            r"\boxed{E = mc^2} \quad \cancel{x}",
            r"A \xrightarrow{f} B \underbrace{c}_{d} \overset{?}{=} e",
            r"\left\{ \frac{x}{2} \middle| x \in \mathbb{Z} \right\}",
        ] {
            let nodes = parse(tex).unwrap();
            let m = mathml(&nodes, true);
            assert!(m.starts_with("<math") && m.ends_with("</math>"), "{tex}");
            assert!(!speech(&nodes).is_empty(), "{tex}");
        }
    }
}
