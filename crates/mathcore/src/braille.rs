//! Nemeth braille: the code blind readers in the US and elsewhere read
//! mathematics in, on refreshable braille displays and embossed pages.
//!
//! Output is Unicode braille (U+2800 block) with ordinary spaces where
//! Nemeth spaces. The core rules are implemented from the 1972 Nemeth Code
//! (with the 2022 BANA guidance on spacing comparison signs):
//!
//! - letters, capitals (dot 6), Greek (dots 46);
//! - digits in the lower cells, the numeric indicator after a space or at
//!   the start, the decimal point;
//! - operation signs; comparison signs spaced on both sides;
//! - grouping symbols;
//! - level indicators for superscripts and subscripts, nested levels, the
//!   numeric-subscript rule (`x_1` is `⠭⠂`), and the return to the baseline;
//! - simple, complex and hypercomplex fraction indicators;
//! - radicals with indices, and the nested-radical prefix;
//! - large operators with limits as modified expressions;
//! - function names followed by a space; primes; ellipsis; text.
//!
//! What it cannot express (an unusual symbol) passes through unchanged
//! rather than being guessed at.

use crate::ast::*;
use crate::symbols::styled_char;

/// A formula in Nemeth braille.
pub fn nemeth(nodes: &[Node]) -> String {
    let mut w = Writer::default();
    w.list(nodes);
    w.out.trim().to_string()
}

#[derive(Default)]
struct Writer {
    out: String,
    /// Open script levels, innermost last: '^' superscript, '_' subscript.
    levels: Vec<char>,
    /// A script ended: the next symbol needs the level it returns to.
    pending_level: bool,
    /// Radicals currently open, for the nested-radical prefix.
    radicals: usize,
    /// Inside a modified expression (limits, accents): numerals there take
    /// no numeric indicator.
    modifiers: usize,
}

const NUMERIC: &str = "⠼";
const CAPITAL: &str = "⠠";
const GREEK: &str = "⠨";
const BASELINE: &str = "⠐";

impl Writer {
    /// Writes braille, first restoring the level after a script. A space
    /// returns to the baseline by itself.
    fn put(&mut self, s: &str) {
        if self.pending_level {
            self.pending_level = false;
            if !s.starts_with(' ') {
                let level = self.level_indicator();
                self.out.push_str(&level);
            }
        }
        self.out.push_str(s);
    }

    fn level_indicator(&self) -> String {
        if self.levels.is_empty() {
            return BASELINE.to_string();
        }
        self.levels.iter().map(|l| if *l == '^' { '⠘' } else { '⠰' }).collect()
    }

    /// A space, collapsing runs. Inside a script, the level is restated
    /// after it, since a space alone returns to the baseline.
    fn space(&mut self) {
        self.pending_level = !self.levels.is_empty();
        if !self.out.is_empty() && !self.out.ends_with(' ') {
            self.out.push(' ');
        }
    }

    fn at_start_or_space(&self) -> bool {
        self.out.is_empty() || self.out.ends_with(' ')
    }

    fn list(&mut self, nodes: &[Node]) {
        for n in nodes {
            self.node(n);
        }
    }

    fn one(&mut self, n: &Node) {
        match n {
            Node::Row(v) => self.list(v),
            _ => self.node(n),
        }
    }

    fn node(&mut self, n: &Node) {
        match n {
            Node::Symbol { ch, atom, variant } => self.symbol(styled_char(*ch, *variant), *atom),
            Node::Row(v) | Node::Style { body: v, .. } | Node::Color { body: v, .. } | Node::Size { body: v, .. } => self.list(v),
            Node::Spanned { body, .. }
            | Node::Class { body, .. }
            | Node::Raise { body, .. }
            | Node::VCenter(body)
            | Node::ColorBox { body, .. }
            | Node::Lap { body, .. }
            | Node::Boxed(body) => self.one(body),
            Node::Choice(b) => self.one(&b[0]),
            Node::Scripts { base, sup, sub } => self.scripts(base, sup.as_deref(), sub.as_deref()),
            Node::BigOp { ch, .. } => self.symbol(*ch, AtomType::Op),
            Node::FnName { name, .. } => {
                self.word(name);
                self.space();
            }
            Node::Frac {
                num, den, rule, delims, ..
            } => {
                if let Some((Some(c), _)) = delims {
                    self.symbol(*c, AtomType::Open);
                }
                if *rule == FracRule::None {
                    // A binomial coefficient: the stack without a fraction line.
                    self.one(num);
                    self.put("⠩");
                    self.one(den);
                } else {
                    let order = frac_order(num).max(frac_order(den));
                    let prefix = CAPITAL.repeat(order);
                    self.put(&format!("{prefix}⠹"));
                    self.one(num);
                    self.put(&format!("{prefix}⠌"));
                    self.one(den);
                    self.put(&format!("{prefix}⠼"));
                }
                if let Some((_, Some(c))) = delims {
                    self.symbol(*c, AtomType::Close);
                }
            }
            Node::Sqrt { radicand, index } => {
                let prefix = GREEK.repeat(self.radicals);
                if let Some(i) = index {
                    self.put("⠣");
                    self.one(i);
                }
                self.put(&format!("{prefix}⠜"));
                self.radicals += 1;
                self.one(radicand);
                self.radicals -= 1;
                self.put(&format!("{prefix}⠻"));
            }
            Node::LeftRight { left, body, right } => {
                if let Some(c) = left {
                    self.symbol(*c, AtomType::Open);
                }
                self.list(body);
                if let Some(c) = right {
                    self.symbol(*c, AtomType::Close);
                }
            }
            Node::Middle(ch) | Node::SizedDelim { ch, .. } => self.symbol(*ch, AtomType::Ord),
            Node::Accent { ch, base, under, .. } => self.modified(base, *ch, *under),
            Node::Overline(base) => self.modified(base, '\u{0304}', false),
            Node::Underline(base) => self.modified(base, '\u{0304}', true),
            Node::Text { text, .. } => self.word(text),
            Node::Space { mu } => {
                if *mu >= 9.0 {
                    self.space();
                }
            }
            Node::Array(a) => {
                for (i, row) in a.rows.iter().enumerate() {
                    if i > 0 {
                        self.out.push('\n');
                        self.pending_level = false;
                    }
                    for (j, cell) in row.iter().enumerate() {
                        if j > 0 {
                            self.space();
                        }
                        self.list(cell);
                    }
                }
            }
            Node::Phantom { .. } | Node::Rule { .. } => {}
            Node::OverUnder { base, over, under } => self.under_over(base, under.as_deref(), over.as_deref()),
            Node::HBrace { base, .. } | Node::Cancel { body: base, .. } => self.one(base),
            Node::XArrow { ch, over, under } => {
                self.space();
                self.put(arrow(*ch));
                // Labels that print nothing (mhchem's spacer) are left out.
                if let Some(o) = over.as_deref().filter(|o| !braille_of(o).is_empty()) {
                    self.put("⠣");
                    self.one(o);
                    self.put("⠻");
                }
                if let Some(u) = under.as_deref().filter(|u| !braille_of(u).is_empty()) {
                    self.put("⠩");
                    self.one(u);
                    self.put("⠻");
                }
                self.space();
            }
            Node::Tagged { body, tag } => {
                self.list(body);
                self.space();
                self.space();
                self.one(tag);
            }
        }
    }

    fn scripts(&mut self, base: &Node, sup: Option<&Node>, sub: Option<&Node>) {
        // Limits above and below a large operator: a modified expression.
        let limits = matches!(base, Node::BigOp { limits, .. } if *limits != Limits::NoLimits)
            && !matches!(base, Node::BigOp { ch, .. } if is_integral(*ch));
        if limits {
            return self.under_over(base, sub, sup);
        }
        self.one(base);
        if let Some(s) = sub {
            // The numeric-subscript rule: a first-level subscript that is a
            // number, on a letter, takes no level indicator (x_1 is ⠭⠂).
            let digits = said_digits(s).filter(|_| self.levels.is_empty() && sup.is_none() && is_letter(base));
            if let Some(d) = digits {
                self.put(&d);
            } else {
                self.levels.push('_');
                let level = self.level_indicator();
                self.put(&level);
                self.one(s);
                self.levels.pop();
                self.pending_level = true;
            }
        }
        if let Some(s) = sup {
            self.pending_level = false;
            self.levels.push('^');
            let level = self.level_indicator();
            self.put(&level);
            self.one(s);
            self.levels.pop();
            self.pending_level = true;
        }
    }

    /// Nemeth's modified expression: ⠐ base ⠩ under ⠣ over ⠻.
    fn under_over(&mut self, base: &Node, under: Option<&Node>, over: Option<&Node>) {
        self.modifiers += 1;
        self.put(BASELINE);
        self.one(base);
        if let Some(u) = under {
            self.put("⠩");
            self.one(u);
        }
        if let Some(o) = over {
            self.put("⠣");
            self.one(o);
        }
        self.put("⠻");
        self.modifiers -= 1;
    }

    /// A base with a mark over or under it (`\bar x`, `\hat x`).
    fn modified(&mut self, base: &Node, mark: char, under: bool) {
        let sign = match mark {
            '\u{0304}' | '\u{0305}' | '¯' => "⠱",
            '\u{0302}' | '^' => "⠸⠣",
            '\u{0303}' | '~' => "⠈⠱",
            '\u{20D7}' | '→' => "⠫⠕",
            '\u{0307}' => "⠡",
            '\u{0308}' => "⠡⠡",
            _ => {
                return self.one(base);
            }
        };
        self.put(BASELINE);
        self.one(base);
        self.put(if under { "⠩" } else { "⠣" });
        self.put(sign);
        self.put("⠻");
    }

    fn word(&mut self, text: &str) {
        for c in text.chars() {
            if c == ' ' {
                self.space();
            } else {
                self.symbol(c, AtomType::Ord);
            }
        }
    }

    fn symbol(&mut self, c: char, atom: AtomType) {
        let c = plain(c);
        if c.is_ascii_digit() || (c == '.' && self.out.ends_with(digit_cell)) {
            let cell = digit(c);
            if c != '.' && self.at_start_or_space() && !self.pending_level && self.modifiers == 0 && self.levels.is_empty() {
                self.put(NUMERIC);
            }
            return self.put(cell);
        }
        if c.is_ascii_alphabetic() {
            let cell = letter(c.to_ascii_lowercase());
            return if c.is_ascii_uppercase() {
                self.put(&format!("{CAPITAL}{cell}"))
            } else {
                self.put(cell)
            };
        }
        if let Some((cell, capital)) = greek(c) {
            return self.put(&format!("{GREEK}{}{cell}", if capital { CAPITAL } else { "" }));
        }
        if let Some(sign) = comparison(c) {
            self.space();
            self.put(sign);
            return self.space();
        }
        if atom == AtomType::Rel {
            // A relation without a Nemeth sign of its own: spaced, as is.
            self.space();
            self.put(&c.to_string());
            return self.space();
        }
        let cell = match c {
            '+' => "⠬",
            '-' | '−' => "⠤",
            '×' => "⠈⠡",
            '÷' => "⠨⠌",
            '⋅' | '·' | '∙' => "⠡",
            '*' | '∗' => "⠈⠼",
            '±' => "⠬⠤",
            '∓' => "⠤⠬",
            '/' => "⠸⠌",
            '(' => "⠷",
            ')' => "⠾",
            '[' => "⠈⠷",
            ']' => "⠈⠾",
            '{' => "⠨⠷",
            '}' => "⠨⠾",
            '⟨' => "⠨⠨⠷",
            '⟩' => "⠨⠨⠾",
            '|' => "⠳",
            '‖' => "⠳⠳",
            '⌊' => "⠈⠰⠷",
            '⌋' => "⠈⠰⠾",
            '⌈' => "⠈⠘⠷",
            '⌉' => "⠈⠘⠾",
            ',' => "⠠",
            ';' => "⠆",
            '!' => "⠯",
            '′' => "⠄",
            '″' => "⠄⠄",
            '‴' => "⠄⠄⠄",
            '∞' => "⠠⠿",
            '∂' => "⠈⠙",
            '∇' => "⠨⠫",
            '∑' => "⠨⠠⠎",
            '∏' => "⠨⠠⠏",
            '∫' => "⠮",
            '∬' => "⠮⠮",
            '∭' => "⠮⠮⠮",
            '∮' => "⠮⠈⠫⠉⠻",
            '∪' => "⠨⠬",
            '∩' => "⠨⠩",
            '∅' => "⠸⠴",
            '…' | '⋯' => "⠄⠄⠄",
            '°' => "⠘⠨⠡",
            '%' => "⠈⠴",
            '∠' => "⠫⠪",
            '△' => "⠫⠞",
            '∘' => "⠨⠡",
            '¬' => "⠈⠹",
            '∀' => "⠈⠯",
            '∃' => "⠈⠿",
            '∧' => "⠈⠩",
            '∨' => "⠈⠬",
            ':' => "⠸⠒",
            '\'' => "⠄",
            _ => {
                let s = c.to_string();
                return self.put(&s);
            }
        };
        self.put(cell);
    }
}

fn braille_of(n: &Node) -> String {
    let mut w = Writer::default();
    w.one(n);
    w.out.trim().to_string()
}

fn digit_cell(c: char) -> bool {
    "⠂⠆⠒⠲⠢⠖⠶⠦⠔⠴".contains(c)
}

fn digit(c: char) -> &'static str {
    match c {
        '1' => "⠂",
        '2' => "⠆",
        '3' => "⠒",
        '4' => "⠲",
        '5' => "⠢",
        '6' => "⠖",
        '7' => "⠶",
        '8' => "⠦",
        '9' => "⠔",
        '0' => "⠴",
        _ => "⠨", // decimal point
    }
}

fn letter(c: char) -> &'static str {
    const CELLS: [&str; 26] = [
        "⠁", "⠃", "⠉", "⠙", "⠑", "⠋", "⠛", "⠓", "⠊", "⠚", "⠅", "⠇", "⠍", "⠝", "⠕", "⠏", "⠟", "⠗", "⠎", "⠞", "⠥", "⠧", "⠺", "⠭", "⠽", "⠵",
    ];
    CELLS[(c as u8 - b'a') as usize]
}

/// Greek letters: the letter after the Greek indicator, and whether it is a capital.
fn greek(c: char) -> Option<(&'static str, bool)> {
    let lower = match c {
        'α' | 'Α' => "⠁",
        'β' | 'Β' => "⠃",
        'γ' | 'Γ' => "⠛",
        'δ' | 'Δ' => "⠙",
        'ε' | 'ϵ' | 'Ε' => "⠑",
        'ζ' | 'Ζ' => "⠵",
        'η' | 'Η' => "⠱",
        'θ' | 'ϑ' | 'Θ' => "⠹",
        'ι' | 'Ι' => "⠊",
        'κ' | 'Κ' => "⠅",
        'λ' | 'Λ' => "⠇",
        'μ' | 'Μ' => "⠍",
        'ν' | 'Ν' => "⠝",
        'ξ' | 'Ξ' => "⠭",
        'ο' | 'Ο' => "⠕",
        'π' | 'ϖ' | 'Π' => "⠏",
        'ρ' | 'ϱ' | 'Ρ' => "⠗",
        'σ' | 'ς' | 'Σ' => "⠎",
        'τ' | 'Τ' => "⠞",
        'υ' | 'Υ' => "⠥",
        'φ' | 'ϕ' | 'Φ' => "⠋",
        'χ' | 'Χ' => "⠯",
        'ψ' | 'Ψ' => "⠽",
        'ω' | 'Ω' => "⠺",
        _ => return None,
    };
    let capital = ('Α'..='Ω').contains(&c);
    Some((lower, capital))
}

/// Comparison signs, which Nemeth spaces on both sides.
fn comparison(c: char) -> Option<&'static str> {
    Some(match c {
        '=' => "⠨⠅",
        '<' => "⠐⠅",
        '>' => "⠨⠂",
        '≤' => "⠐⠅⠱",
        '≥' => "⠨⠂⠱",
        '≠' => "⠌⠨⠅",
        '≈' => "⠈⠱⠈⠱",
        '≡' => "⠸⠇",
        '∼' => "⠈⠱",
        '≅' => "⠈⠱⠨⠅",
        '∝' => "⠸⠿",
        '∈' => "⠈⠑",
        '∉' => "⠌⠈⠑",
        '⊂' => "⠸⠐⠅",
        '⊆' => "⠸⠐⠅⠱",
        '⊃' => "⠸⠨⠂",
        '⊇' => "⠸⠨⠂⠱",
        '→' | '⟶' => "⠫⠕",
        '←' => "⠫⠪",
        '↔' | '⇔' => "⠫⠪⠒⠒⠕",
        '⇒' => "⠫⠶⠕",
        '↦' => "⠫⠳⠒⠒⠕",
        '∣' => "⠳",
        '∥' => "⠫⠇",
        '⊥' => "⠫⠏",
        _ => return None,
    })
}

fn arrow(ch: char) -> &'static str {
    match ch {
        '←' => "⠫⠪",
        '↔' => "⠫⠪⠒⠒⠕",
        '⇌' | '⇋' | '⇄' => "⠫⠒⠒⠕⠫⠪⠒⠒",
        '=' => "⠨⠅",
        _ => "⠫⠕",
    }
}

fn is_integral(c: char) -> bool {
    matches!(c, '∫' | '∬' | '∭' | '∮')
}

/// Undoes the math alphabets: 𝑥 is read as x.
fn plain(c: char) -> char {
    crate::a11y::plain_char(c)
}

fn is_letter(n: &Node) -> bool {
    match n {
        Node::Symbol { ch, .. } => plain(*ch).is_ascii_alphabetic() || greek(*ch).is_some(),
        Node::Spanned { body, .. } => is_letter(body),
        Node::Row(v) if v.len() == 1 => is_letter(&v[0]),
        _ => false,
    }
}

/// The braille digits of a node that is only a number, else None.
fn said_digits(n: &Node) -> Option<String> {
    let mut out = String::new();
    fn walk(n: &Node, out: &mut String) -> bool {
        match n {
            Node::Symbol { ch, .. } if ch.is_ascii_digit() => {
                out.push_str(digit(*ch));
                true
            }
            Node::Row(v) => v.iter().all(|c| walk(c, out)),
            Node::Spanned { body, .. } => walk(body, out),
            _ => false,
        }
    }
    (walk(n, &mut out) && !out.is_empty()).then_some(out)
}

/// 0 for a fraction with no fraction inside it, 1 for a complex fraction
/// (one inside), 2 for hypercomplex, and so on.
fn frac_order(n: &Node) -> usize {
    fn deepest(n: &Node) -> usize {
        match n {
            Node::Frac { num, den, rule, .. } if *rule != FracRule::None => 1 + deepest(num).max(deepest(den)),
            Node::Row(v) | Node::Style { body: v, .. } | Node::Color { body: v, .. } | Node::Size { body: v, .. } => {
                v.iter().map(deepest).max().unwrap_or(0)
            }
            Node::LeftRight { body, .. } => body.iter().map(deepest).max().unwrap_or(0),
            Node::Spanned { body, .. } | Node::Class { body, .. } => deepest(body),
            Node::Scripts { base, sup, sub } => deepest(base)
                .max(sup.as_deref().map_or(0, deepest))
                .max(sub.as_deref().map_or(0, deepest)),
            Node::Sqrt { radicand, .. } => deepest(radicand),
            _ => 0,
        }
    }
    deepest(n)
}

#[cfg(test)]
mod tests {
    use super::nemeth;
    use crate::parse;

    fn nm(tex: &str) -> String {
        nemeth(&parse(tex).unwrap())
    }

    #[test]
    fn numbers_letters_and_operations() {
        assert_eq!(nm("3+5=8"), "⠼⠒⠬⠢ ⠨⠅ ⠼⠦");
        assert_eq!(nm("x+y"), "⠭⠬⠽");
        assert_eq!(nm("2.5"), "⠼⠆⠨⠢");
        assert_eq!(nm("A"), "⠠⠁");
        assert_eq!(nm(r"\alpha + \Omega"), "⠨⠁⠬⠨⠠⠺");
        assert_eq!(nm("a < b"), "⠁ ⠐⠅ ⠃");
        assert_eq!(nm("(x)"), "⠷⠭⠾");
    }

    #[test]
    fn levels() {
        assert_eq!(nm("x^2"), "⠭⠘⠆");
        assert_eq!(nm("x^2+y^2=z^2"), "⠭⠘⠆⠐⠬⠽⠘⠆ ⠨⠅ ⠵⠘⠆");
        assert_eq!(nm("x_1"), "⠭⠂");
        assert_eq!(nm("x_1 + x_2"), "⠭⠂⠬⠭⠆");
        assert_eq!(nm("x_{i}"), "⠭⠰⠊");
        assert_eq!(nm("e^{x^2}"), "⠑⠘⠭⠘⠘⠆");
        assert_eq!(nm("e^{x^2} + 1"), "⠑⠘⠭⠘⠘⠆⠐⠬⠂");
        // A comparison inside a script is spaced and the level restated.
        assert_eq!(nm("x^{a=b}"), "⠭⠘⠁ ⠘⠨⠅ ⠘⠃");
    }

    #[test]
    fn fractions_and_radicals() {
        assert_eq!(nm(r"\frac{1}{2}"), "⠹⠂⠌⠆⠼");
        assert_eq!(nm(r"\frac{a+b}{c}"), "⠹⠁⠬⠃⠌⠉⠼");
        assert_eq!(nm(r"\frac{\frac{1}{2}}{3}"), "⠠⠹⠹⠂⠌⠆⠼⠠⠌⠒⠠⠼");
        assert_eq!(nm(r"\sqrt{x}"), "⠜⠭⠻");
        assert_eq!(nm(r"\sqrt[3]{x}"), "⠣⠒⠜⠭⠻");
        assert_eq!(nm(r"\sqrt{\sqrt{x}}"), "⠜⠨⠜⠭⠨⠻⠻");
    }

    #[test]
    fn operators_functions_and_marks() {
        assert_eq!(nm(r"\sum_{i=1}^{n} i"), "⠐⠨⠠⠎⠩⠊ ⠨⠅ ⠂⠣⠝⠻⠊");
        assert_eq!(nm(r"\sin x"), "⠎⠊⠝ ⠭");
        assert_eq!(nm(r"\bar{x}"), "⠐⠭⠣⠱⠻");
        assert_eq!(nm(r"f'(x)"), "⠋⠘⠄⠐⠷⠭⠾");
        assert_eq!(nm(r"\infty"), "⠠⠿");
    }

    #[test]
    fn every_corpus_formula_produces_braille() {
        for tex in [
            r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
            r"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}",
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
            r"\ce{2H2 + O2 -> 2H2O}",
            r"\text{if } x > 0",
        ] {
            assert!(!nm(tex).contains("⠣⠻"), "{tex}");
            assert!(!nm(tex).is_empty(), "{tex}");
        }
    }
}
