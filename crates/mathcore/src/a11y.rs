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
//! "the square root of" for a radical, at one of three verbosity levels.
//!
//! `speech_tree` breaks the same reading into a tree a screen reader can walk
//! part by part, each part carrying the source range a host highlights while
//! it is read.

use crate::ast::*;
pub use crate::speech_lang::Language;
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
    speech_with(nodes, &SpeechOptions::default())
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
        Node::Size { factor, body } => {
            let _ = write!(out, r#"<mstyle mathsize="{:.0}%">"#, factor * 100.0);
            row(out, body);
            out.push_str("</mstyle>");
        }
        Node::Tagged { body, tag } => {
            out.push_str("<mtable><mlabeledtr><mtd>");
            one(out, tag);
            out.push_str("</mtd><mtd>");
            row(out, body);
            out.push_str("</mtd></mlabeledtr></mtable>");
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

/// How much scaffolding the spoken text carries.
///
/// The levels follow MathSpeak's: a listener new to a formula wants to hear
/// where every fraction and root ends, one who knows it wants the content and
/// nothing else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Verbosity {
    /// Every structure is opened and closed: "the fraction 1 over 2, end
    /// fraction". Nothing is ambiguous, at the cost of length.
    Verbose,
    /// Scaffolding only where the reading would otherwise be ambiguous:
    /// "1 over 2", but "the fraction a plus b over c,".
    #[default]
    Brief,
    /// Content words only: "sum from i equals 1 to n of i".
    Superbrief,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SpeechOptions {
    pub verbosity: Verbosity,
    pub language: Language,
}

/// One step of a spoken walk through a formula.
///
/// A screen reader starts at the root, which speaks the whole formula, and
/// moves down into the children: the numerator and denominator of a fraction,
/// the base and scripts of a power, the cells of a matrix. `start..end` is the
/// smallest recorded range of source covering the node, which a host passes to
/// `DisplayList::highlight` so the part being read lights up as it is read.
#[derive(Debug, Clone, PartialEq)]
pub struct SpeechNode {
    /// What the node is: "fraction", "root", "scripts", "matrix", "symbol", ...
    pub role: &'static str,
    /// Its place in the parent: "numerator", "superscript", "row 2", or empty.
    pub label: String,
    /// The node spoken on its own.
    pub text: String,
    pub start: u32,
    pub end: u32,
    pub children: Vec<SpeechNode>,
}

impl SpeechNode {
    /// The tree as JSON, for bindings that hand it to a host language whole.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        self.write_json(&mut out);
        out
    }

    fn write_json(&self, out: &mut String) {
        out.push_str(r#"{"role":"#);
        json_str(out, self.role);
        out.push_str(r#","label":"#);
        json_str(out, &self.label);
        out.push_str(r#","text":"#);
        json_str(out, &self.text);
        let _ = write!(out, r#","start":{},"end":{},"children":["#, self.start, self.end);
        for (i, c) in self.children.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            c.write_json(out);
        }
        out.push_str("]}");
    }
}

fn json_str(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A spoken rendering of a parsed formula at the given verbosity.
pub fn speech_with(nodes: &[Node], opts: &SpeechOptions) -> String {
    let sp = Speaker {
        v: opts.verbosity,
        lang: opts.language,
    };
    let mut out = String::new();
    sp.say_list(&mut out, nodes);
    tidy(&out)
}

/// The formula as a tree a screen reader can walk. Source ranges are only
/// recorded when the nodes come from `parse_with_spans`; otherwise every
/// range is `0..0`.
pub fn speech_tree(nodes: &[Node], opts: &SpeechOptions) -> SpeechNode {
    let sp = Speaker {
        v: opts.verbosity,
        lang: opts.language,
    };
    let mut root = sp.tree_list(nodes, String::new());
    if root.role == "row" {
        root.role = "formula";
    }
    let span = root_span(&root);
    fill_spans(&mut root, span);
    root
}

fn tidy(s: &str) -> String {
    let text = s.split_whitespace().collect::<Vec<_>>().join(" ");
    text.replace(" ,", ",")
}

/// Raw tree node: a span only where the source recorded one.
fn root_span(n: &SpeechNode) -> (u32, u32) {
    if n.end > n.start {
        (n.start, n.end)
    } else {
        (0, 0)
    }
}

/// Ranges are built bottom-up (a group covers its parts) and then filled
/// top-down, so a part whose own position was not recorded gets its parent's.
fn fill_spans(n: &mut SpeechNode, parent: (u32, u32)) {
    if n.end <= n.start {
        n.start = parent.0;
        n.end = parent.1;
    }
    let me = (n.start, n.end);
    for c in &mut n.children {
        fill_spans(c, me);
    }
}

struct Speaker {
    v: Verbosity,
    lang: Language,
}

fn word(out: &mut String, w: &str) {
    if !out.is_empty() && !out.ends_with(' ') {
        out.push(' ');
    }
    out.push_str(w);
}

impl Speaker {
    fn verbose(&self) -> bool {
        self.v == Verbosity::Verbose
    }

    fn superbrief(&self) -> bool {
        self.v == Verbosity::Superbrief
    }

    /// A phrase with its leading article dropped at superbrief.
    fn the<'a>(&self, phrase: &'a str) -> &'a str {
        if self.superbrief() {
            phrase.strip_prefix("the ").unwrap_or(phrase)
        } else {
            phrase
        }
    }

    fn say_list(&self, out: &mut String, nodes: &[Node]) {
        for n in nodes {
            self.say(out, n);
        }
    }

    fn say_one(&self, out: &mut String, n: &Node) {
        match n {
            Node::Row(v) => self.say_list(out, v),
            _ => self.say(out, n),
        }
    }

    /// Renders a node to a fresh string, to test how short it is.
    fn said(&self, n: &Node) -> String {
        let mut s = String::new();
        self.say_one(&mut s, n);
        s.trim().to_string()
    }

    /// True for something short enough to read without "the ... of" scaffolding.
    fn simple(&self, n: &Node) -> bool {
        let s = self.said(n);
        !s.is_empty() && s.split_whitespace().count() == 1
    }

    /// Closes a structure: always at verbose, after anything longer than a
    /// word at brief, never at superbrief.
    /// Speaks an English phrase in the chosen language.
    fn w(&self, out: &mut String, english: &str) {
        word(out, &crate::speech_lang::translate(self.lang, english));
    }

    /// A phrase with numbers or words in it: `{}` in the template, filled in order.
    fn template(&self, out: &mut String, english: &str, args: &[&str]) {
        let mut t = crate::speech_lang::translate(self.lang, english);
        for a in args {
            t = t.replacen("{}", a, 1);
        }
        word(out, &t);
    }

    fn close(&self, out: &mut String, simple: bool, end: &str) {
        if self.verbose() {
            word(out, ",");
            self.w(out, end);
        } else if !self.superbrief() && !simple {
            word(out, ",");
        }
    }

    fn say(&self, out: &mut String, n: &Node) {
        match n {
            Node::Symbol { ch, variant, .. } => self.w(out, &symbol_name(styled_char(*ch, *variant))),
            Node::Row(v) => self.say_list(out, v),
            Node::Scripts { base, sup, sub } => {
                let limits = matches!(**base, Node::BigOp { .. } | Node::FnName { .. } | Node::HBrace { .. });
                if limits {
                    self.say_one(out, base);
                    if let Some(s) = sub {
                        self.w(out, "from");
                        self.say_one(out, s);
                    }
                    if let Some(s) = sup {
                        self.w(out, "to");
                        self.say_one(out, s);
                    }
                    self.w(out, "of");
                    return;
                }
                self.say_one(out, base);
                if let Some(s) = sub {
                    self.w(out, "sub");
                    self.say_one(out, s);
                    if self.verbose() && sup.is_none() {
                        self.w(out, ", end sub");
                    }
                }
                if let Some(s) = sup {
                    match self.said(s).as_str() {
                        "2" => self.w(out, "squared"),
                        "3" => self.w(out, "cubed"),
                        _ if self.simple(s) => {
                            self.w(out, "to the");
                            self.say_one(out, s);
                        }
                        _ => {
                            self.w(out, if self.superbrief() { "to the" } else { "to the power of" });
                            self.say_one(out, s);
                            self.close(out, false, "end power");
                        }
                    }
                }
            }
            Node::BigOp { ch, .. } => self.w(out, self.the(&big_op_name(*ch))),
            Node::FnName { name, .. } => word(out, name),
            Node::Frac {
                num, den, rule, delims, ..
            } => {
                if delims.is_some() && *rule == FracRule::None {
                    self.say_one(out, num);
                    self.w(out, "choose");
                    self.say_one(out, den);
                    return;
                }
                let short = self.simple(num) && self.simple(den);
                if self.verbose() || (!short && !self.superbrief()) {
                    self.w(out, "the fraction");
                }
                self.say_one(out, num);
                self.w(out, "over");
                self.say_one(out, den);
                self.close(out, short, "end fraction");
            }
            Node::Sqrt { radicand, index } => {
                match index {
                    Some(i) => {
                        let index = self.said(i);
                        if index == "3" {
                            self.w(out, self.the("the cube root of"));
                        } else {
                            self.template(out, self.the("the {}th root of"), &[&index]);
                        }
                    }
                    None => self.w(out, self.the("the square root of")),
                }
                self.say_one(out, radicand);
                self.close(out, self.simple(radicand), "end root");
            }
            Node::LeftRight { left, body, right } => {
                if let Some(c) = left {
                    self.w(out, self.the(open_name(*c)));
                }
                self.say_list(out, body);
                if let Some(c) = right {
                    let close = close_name(*c);
                    if !close.is_empty() {
                        self.w(out, close);
                    } else if self.verbose() {
                        let name = crate::speech_lang::translate(self.lang, delimited_name(left.unwrap_or(*c)));
                        self.template(out, ", end {}", &[&name]);
                    }
                }
            }
            Node::Middle(ch) | Node::SizedDelim { ch, .. } => self.w(out, &symbol_name(*ch)),
            Node::Accent { ch, base, .. } => {
                self.say_one(out, base);
                self.w(out, accent_name(*ch));
            }
            Node::Overline(inner) => {
                self.say_one(out, inner);
                self.w(out, "bar");
            }
            Node::Underline(inner) => {
                self.say_one(out, inner);
                self.w(out, "underlined");
            }
            Node::Style { body, .. } | Node::Size { body, .. } => self.say_list(out, body),
            Node::Tagged { body, tag } => {
                self.say_list(out, body);
                self.template(out, ", equation {}", &[self.text_of(tag).trim_matches(['(', ')', ' '])]);
            }
            Node::Color { body, .. } => self.say_list(out, body),
            Node::Text { text, .. } => word(out, text.trim()),
            Node::Space { .. } => {}
            Node::Array(a) => {
                let rows = a.rows.len();
                let cols = a.rows.iter().map(|r| r.len()).max().unwrap_or(0);
                if cols > 1 {
                    let (r, c) = (rows.to_string(), cols.to_string());
                    self.template(out, self.the("the {} by {} matrix,"), &[&r, &c]);
                } else {
                    self.template(out, "{} rows,", &[&rows.to_string()]);
                }
                for (i, r) in a.rows.iter().enumerate() {
                    self.template(out, "row {},", &[&(i + 1).to_string()]);
                    for (j, cell) in r.iter().enumerate() {
                        if j > 0 {
                            word(out, ",");
                        }
                        let before = out.len();
                        self.say_list(out, cell);
                        if out[before..].trim().is_empty() {
                            self.w(out, "blank");
                        }
                    }
                    word(out, ",");
                }
                if self.verbose() {
                    self.w(out, if cols > 1 { "end matrix" } else { "end rows" });
                }
            }
            Node::Phantom { .. } => {}
            Node::OverUnder { base, over, under } => {
                self.say_one(out, base);
                if let Some(u) = under {
                    self.w(out, "under");
                    self.say_one(out, u);
                }
                if let Some(o) = over {
                    self.w(out, "over");
                    self.say_one(out, o);
                }
            }
            Node::Boxed(inner) => {
                self.w(out, "boxed,");
                self.say_one(out, inner);
                if self.verbose() {
                    self.w(out, ", end box");
                }
            }
            Node::Cancel { body, .. } => {
                self.w(out, "crossed out,");
                self.say_one(out, body);
                if self.verbose() {
                    self.w(out, ", end crossed out");
                }
            }
            Node::HBrace { base, over } => {
                self.w(out, if *over { "over brace of" } else { "under brace of" });
                self.say_one(out, base);
            }
            Node::XArrow { ch, over, under } => {
                self.w(out, &symbol_name(*ch));
                // An empty label (`\xrightarrow{}`, a bare `->` in \ce) says nothing.
                if let Some(o) = over.as_deref().filter(|o| !self.said(o).is_empty()) {
                    self.w(out, "with");
                    self.say_one(out, o);
                    self.w(out, "above,");
                }
                if let Some(u) = under.as_deref().filter(|u| !self.said(u).is_empty()) {
                    self.w(out, "with");
                    self.say_one(out, u);
                    self.w(out, "below,");
                }
            }
            Node::Class { body, .. } => self.say_one(out, body),
            Node::Raise { body, .. } | Node::VCenter(body) | Node::ColorBox { body, .. } | Node::Lap { body, .. } => {
                self.say_one(out, body)
            }
            Node::Choice(b) => self.say_one(out, &b[0]),
            Node::Rule { .. } => {}
            Node::Spanned { body, .. } => self.say_one(out, body),
        }
    }

    // ---- navigation tree ----

    fn text_of(&self, n: &Node) -> String {
        let mut s = String::new();
        self.say_one(&mut s, n);
        tidy(&s)
    }

    fn leaf(&self, role: &'static str, label: String, n: &Node) -> SpeechNode {
        SpeechNode {
            role,
            label,
            text: self.text_of(n),
            start: 0,
            end: 0,
            children: Vec::new(),
        }
    }

    fn branch(&self, role: &'static str, label: String, n: &Node, parts: Vec<(&str, &Node)>) -> SpeechNode {
        let children = parts
            .into_iter()
            .map(|(l, c)| self.tree(c, l.to_string()))
            .filter(|c| !c.text.is_empty())
            .collect();
        with_children(self.leaf(role, label, n), children)
    }

    fn tree_list(&self, nodes: &[Node], label: String) -> SpeechNode {
        if nodes.len() == 1 {
            return self.tree(&nodes[0], label);
        }
        let mut s = String::new();
        self.say_list(&mut s, nodes);
        let children = nodes
            .iter()
            .map(|c| self.tree(c, String::new()))
            .filter(|c| !c.text.is_empty())
            .collect();
        with_children(
            SpeechNode {
                role: "row",
                label,
                text: tidy(&s),
                start: 0,
                end: 0,
                children,
            },
            Vec::new(),
        )
    }

    fn tree(&self, n: &Node, label: String) -> SpeechNode {
        match n {
            Node::Spanned { span, body } => {
                let mut t = self.tree(body, label);
                t.start = span.start;
                t.end = span.end;
                t
            }
            Node::Row(v) => self.tree_list(v, label),
            Node::Style { body, .. } | Node::Color { body, .. } | Node::Size { body, .. } => self.tree_list(body, label),
            Node::Tagged { body, tag } => {
                let mut formula = self.tree_list(body, String::new());
                let mut number = self.tree(tag, "equation number".to_string());
                number.text = number.text.trim_matches(['(', ')', ' ']).to_string();
                let text = tidy(&format!("{}, equation {}", formula.text, number.text));
                if formula.role == "row" {
                    formula.children.push(number);
                    formula.text = text;
                    formula.label = label;
                    formula
                } else {
                    with_children(
                        SpeechNode {
                            role: "row",
                            label,
                            text,
                            start: 0,
                            end: 0,
                            children: Vec::new(),
                        },
                        vec![formula, number],
                    )
                }
            }
            Node::Class { body, .. }
            | Node::Raise { body, .. }
            | Node::VCenter(body)
            | Node::ColorBox { body, .. }
            | Node::Lap { body, .. } => self.tree(body, label),
            Node::Choice(b) => self.tree(&b[0], label),
            Node::Scripts { base, sup, sub } => {
                let limits = matches!(**base, Node::BigOp { .. } | Node::FnName { .. } | Node::HBrace { .. });
                let (lo, hi) = if limits {
                    ("lower limit", "upper limit")
                } else {
                    ("subscript", "superscript")
                };
                let mut parts = vec![("base", &**base)];
                if let Some(s) = sub {
                    parts.push((lo, &**s));
                }
                if let Some(s) = sup {
                    parts.push((hi, &**s));
                }
                self.branch("scripts", label, n, parts)
            }
            Node::Frac { num, den, .. } => self.branch("fraction", label, n, vec![("numerator", &**num), ("denominator", &**den)]),
            Node::Sqrt { radicand, index } => {
                let mut parts = Vec::new();
                if let Some(i) = index {
                    parts.push(("index", &**i));
                }
                parts.push(("radicand", &**radicand));
                self.branch("root", label, n, parts)
            }
            Node::LeftRight { body, .. } => {
                let inner = self.tree_list(body, "contents".to_string());
                let parts = if inner.text.is_empty() { Vec::new() } else { vec![inner] };
                with_children(self.leaf("delimited", label, n), parts)
            }
            Node::Accent { base, .. } | Node::Overline(base) | Node::Underline(base) | Node::HBrace { base, .. } => {
                self.branch("accent", label, n, vec![("base", &**base)])
            }
            Node::Boxed(body) | Node::Cancel { body, .. } => self.branch("enclosure", label, n, vec![("contents", &**body)]),
            Node::OverUnder { base, over, under } => {
                let mut parts = vec![("base", &**base)];
                if let Some(u) = under {
                    parts.push(("under", &**u));
                }
                if let Some(o) = over {
                    parts.push(("over", &**o));
                }
                self.branch("stack", label, n, parts)
            }
            Node::XArrow { over, under, .. } => {
                let mut parts = Vec::new();
                if let Some(o) = over {
                    parts.push(("over", &**o));
                }
                if let Some(u) = under {
                    parts.push(("under", &**u));
                }
                self.branch("arrow", label, n, parts)
            }
            Node::Array(a) => {
                let rows = a
                    .rows
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        let cells: Vec<SpeechNode> = r
                            .iter()
                            .enumerate()
                            .map(|(j, cell)| {
                                let mut c = self.tree_list(cell, format!("column {}", j + 1));
                                c.role = "cell";
                                if c.text.is_empty() {
                                    c.text = "blank".to_string();
                                }
                                c
                            })
                            .collect();
                        let text = tidy(&cells.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join(" , "));
                        with_children(
                            SpeechNode {
                                role: "table row",
                                label: format!("row {}", i + 1),
                                text,
                                start: 0,
                                end: 0,
                                children: Vec::new(),
                            },
                            cells,
                        )
                    })
                    .collect();
                with_children(self.leaf("matrix", label, n), rows)
            }
            Node::BigOp { .. } => self.leaf("operator", label, n),
            Node::FnName { .. } => self.leaf("function", label, n),
            Node::Text { .. } => self.leaf("text", label, n),
            _ => self.leaf("symbol", label, n),
        }
    }
}

/// Attaches children and gives the node the range they cover. A node with a
/// single child that says the same thing collapses into it, so navigation
/// never stops twice on one thing.
fn with_children(mut n: SpeechNode, children: Vec<SpeechNode>) -> SpeechNode {
    if !children.is_empty() {
        n.children = children;
    }
    if n.end <= n.start {
        let recorded = || n.children.iter().filter(|c| c.end > c.start);
        let start = recorded().map(|c| c.start).min();
        let end = recorded().map(|c| c.end).max();
        if let (Some(s), Some(e)) = (start, end) {
            n.start = s;
            n.end = e;
        }
    }
    if n.children.len() == 1 && n.children[0].text == n.text {
        let mut only = n.children.pop().unwrap();
        if !n.label.is_empty() {
            only.label = n.label;
        }
        if only.end <= only.start {
            only.start = n.start;
            only.end = n.end;
        }
        return only;
    }
    n
}

/// The structure a pair of bars or brackets forms, for "end absolute value".
fn delimited_name(c: char) -> &'static str {
    match c {
        '|' => "absolute value",
        '‖' => "norm",
        '⌊' | '⌋' => "floor",
        '⌈' | '⌉' => "ceiling",
        _ => "group",
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
        '←' => "comes from",
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

/// A character with its math alphabet undone: 𝑥 is x, 𝜶 is α.
pub(crate) fn plain_char(c: char) -> char {
    plain_letter(c).or_else(|| plain_greek(c)).unwrap_or(c)
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

    fn spv(tex: &str, verbosity: Verbosity) -> String {
        speech_with(
            &parse(tex).unwrap(),
            &SpeechOptions {
                verbosity,
                ..Default::default()
            },
        )
    }

    #[test]
    fn verbosity_levels() {
        use Verbosity::*;
        assert_eq!(spv(r"\frac{1}{2}", Verbose), "the fraction 1 over 2, end fraction");
        assert_eq!(spv(r"\frac{1}{2}", Brief), "1 over 2");
        assert_eq!(spv(r"\frac{a+b}{c}", Superbrief), "a plus b over c");
        assert_eq!(spv(r"\sqrt{x+1}", Verbose), "the square root of x plus 1, end root");
        assert_eq!(spv(r"\sqrt{x+1}", Superbrief), "square root of x plus 1");
        assert_eq!(spv(r"x^{n+1}", Verbose), "x to the power of n plus 1, end power");
        assert_eq!(spv(r"x^{n+1}", Superbrief), "x to the n plus 1");
        assert_eq!(spv(r"\sum_{i=1}^{n} i", Superbrief), "sum from i equals 1 to n of i");
        assert_eq!(spv(r"\left| x \right|", Verbose), "the absolute value of x, end absolute value");
        assert_eq!(spv(r"x_i", Verbose), "x sub i, end sub");
        // Brief is the default and unchanged.
        assert_eq!(spv("x^2 + y^2 = z^2", Brief), sp("x^2 + y^2 = z^2"));
    }

    fn tree(tex: &str) -> SpeechNode {
        crate::render_speech_tree(tex, &crate::Macros::new(), &SpeechOptions::default()).unwrap()
    }

    #[test]
    fn tree_walks_down_into_parts() {
        let src = r"\frac{a+b}{c} = 1";
        let t = tree(src);
        assert_eq!(t.role, "formula");
        assert_eq!(t.text, "the fraction a plus b over c, equals 1");
        assert_eq!((t.start, t.end), (0, src.len() as u32));
        let frac = &t.children[0];
        assert_eq!(frac.role, "fraction");
        assert_eq!(&src[frac.start as usize..frac.end as usize], r"\frac{a+b}{c}");
        let num = &frac.children[0];
        assert_eq!((num.label.as_str(), num.text.as_str()), ("numerator", "a plus b"));
        assert_eq!(&src[num.start as usize..num.end as usize], "a+b");
        assert_eq!(num.children.len(), 3);
        assert_eq!(&src[num.children[2].start as usize..num.children[2].end as usize], "b");
        let den = &frac.children[1];
        assert_eq!((den.label.as_str(), den.text.as_str()), ("denominator", "c"));
        assert_eq!(&src[den.start as usize..den.end as usize], "c");
    }

    #[test]
    fn tree_labels_scripts_matrices_and_limits() {
        let t = tree(r"\sum_{i=1}^{n} x_i");
        let sum = &t.children[0];
        let labels: Vec<_> = sum.children.iter().map(|c| c.label.as_str()).collect();
        assert_eq!(labels, ["base", "lower limit", "upper limit"]);

        let t = tree(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}");
        let m = t.children.iter().find(|c| c.role == "matrix").unwrap_or(&t);
        let m = if m.role == "delimited" { &m.children[0] } else { m };
        assert_eq!(m.role, "matrix");
        assert_eq!(m.children.len(), 2);
        assert_eq!(m.children[1].label, "row 2");
        assert_eq!(m.children[1].children[0].text, "c");
        assert_eq!(m.children[1].children[1].label, "column 2");
    }

    #[test]
    fn tree_never_stops_twice_on_one_thing() {
        fn check(n: &SpeechNode) {
            assert!(!(n.children.len() == 1 && n.children[0].text == n.text), "{n:?}");
            assert!(n.start <= n.end);
            for c in &n.children {
                assert!(c.start >= n.start && c.end <= n.end, "{c:?} outside {n:?}");
                check(c);
            }
        }
        for tex in [r"x", r"{{x}}", r"\color{red}{x+y}", r"\left( \frac{1}{2} \right)", r"\sqrt[3]{x^2}"] {
            check(&tree(tex));
        }
    }

    #[test]
    fn empty_parts_are_spoken_not_skipped() {
        // Found by the arXiv corpus check: empty cells and empty delimiter
        // pairs produced navigation stops with nothing to say.
        assert!(sp(r"\begin{array}{cc} & 1 \\ -1 & \end{array}").contains("row 1, blank, 1"));
        fn check(n: &SpeechNode) {
            assert!(!n.text.is_empty(), "{n:?}");
            n.children.iter().for_each(check);
        }
        check(&tree(r"\left( \begin{array}{cc} & 1\\-1 & \end{array}\right)"));
        check(&tree(r"(m + n - 1)\left|\right>_i = 0"));
        check(&tree(r"\begin{array}{c} p_1 \\ \\ c_1 \end{array}"));
    }

    #[test]
    fn empty_arrow_labels_are_silent() {
        assert_eq!(sp(r"\ce{2H2 + O2 -> 2H2O}"), "2 H sub 2 plus O sub 2 goes to 2 H sub 2 O");
        assert_eq!(sp(r"A \xrightarrow{f} B"), "A goes to with f above, B");
        assert_eq!(sp(r"A \xrightarrow[g]{f} B"), "A goes to with f above, with g below, B");
    }

    #[test]
    fn tree_json_is_well_formed() {
        let j = tree(r#"\text{say "hi"} x"#).to_json();
        assert!(j.starts_with(r#"{"role":"formula""#));
        assert!(j.contains(r#"say \"hi\""#));
        assert_eq!(j.matches('{').count(), j.matches('}').count());
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
