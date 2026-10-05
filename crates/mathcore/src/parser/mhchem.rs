//! Chemistry: `\ce{...}` and `\pu{...}` from the mhchem package.
//!
//! mhchem's notation is translated to ordinary TeX, which the main parser
//! then reads, so chemistry gets layout, line breaking, hit testing and
//! speech for free. The translation covers what chemistry teaching uses:
//!
//! - formulas: `H2O`, `(NH4)2SO4`, `CuSO4*5H2O`, coefficients `2H2`;
//! - charges and oxidation states: `Na+`, `SO4^2-`, `Fe^{III}`, `OH-`;
//! - isotopes: `^{227}_{90}Th`;
//! - arrows, with text above and below: `->`, `<-`, `<->`, `<=>`, `<=>>`,
//!   `<<=>`, `<-->`, `->[\Delta][cat.]`;
//! - gas and precipitate: ` ^` and ` v`;
//! - bonds inside a formula: `-`, `=`, `#`;
//! - states `(aq)`, plain words, `$math$` and TeX commands passed through;
//! - `\pu{123 kJ/mol}`: a number with upright units.

/// Translates the argument of `\ce` to TeX.
pub(super) fn ce_to_tex(src: &str) -> Result<String, String> {
    let chars: Vec<char> = src.chars().collect();
    let mut t = Translator {
        c: &chars,
        i: 0,
        out: String::new(),
        in_formula: false,
    };
    t.run()?;
    Ok(t.out)
}

/// Translates the argument of `\pu` to TeX.
pub(super) fn pu_to_tex(src: &str) -> String {
    let mut out = String::new();
    let mut chars = src.trim().chars().peekable();
    // The number: digits, a decimal point or comma, an exponent.
    let mut number = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() || matches!(c, '.' | ',' | '-' | '+' | '(' | ')') || (c == 'e' && !number.is_empty()) {
            number.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if let Some(exp) = number.find('e') {
        out.push_str(&number[..exp]);
        out.push_str(r"\times 10^{");
        out.push_str(&number[exp + 1..]);
        out.push('}');
    } else {
        out.push_str(&number);
    }
    let unit: String = chars.collect();
    let unit = unit.trim();
    if unit.is_empty() {
        return out;
    }
    if !number.is_empty() {
        out.push_str(r"\,");
    }
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        if !word.is_empty() {
            out.push_str(r"\mathrm{");
            out.push_str(word);
            out.push('}');
            word.clear();
        }
    };
    let mut it = unit.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            c if c.is_alphabetic() || c == '°' || c == 'µ' => word.push(c),
            '.' | '*' | ' ' => {
                flush(&mut word, &mut out);
                out.push_str(r"\,");
            }
            '^' => {
                flush(&mut word, &mut out);
                out.push_str("^{");
                while let Some(&d) = it.peek() {
                    if d.is_ascii_digit() || d == '-' || d == '+' {
                        out.push(d);
                        it.next();
                    } else {
                        break;
                    }
                }
                out.push('}');
            }
            '/' => {
                flush(&mut word, &mut out);
                out.push('/');
            }
            c => {
                flush(&mut word, &mut out);
                out.push(c);
            }
        }
    }
    flush(&mut word, &mut out);
    out
}

struct Translator<'s> {
    c: &'s [char],
    i: usize,
    out: String,
    /// Just after an element or a closing bracket: digits are subscripts and
    /// a trailing `+` or `-` is a charge.
    in_formula: bool,
}

const ARROWS: &[(&str, &str)] = &[
    ("<=>>", r"\xrightleftharpoons"),
    ("<<=>", r"\xrightleftharpoons"),
    ("<-->", r"\xrightleftarrows"),
    ("<=>", r"\xrightleftharpoons"),
    ("<->", r"\xleftrightarrow"),
    ("->", r"\xrightarrow"),
    ("<-", r"\xleftarrow"),
];

impl Translator<'_> {
    fn peek(&self, k: usize) -> Option<char> {
        self.c.get(self.i + k).copied()
    }

    fn at(&self, s: &str) -> bool {
        s.chars().enumerate().all(|(k, ch)| self.peek(k) == Some(ch))
    }

    /// End of a formula: what may follow a charge.
    fn boundary(&self, k: usize) -> bool {
        matches!(self.peek(k), None | Some(' ' | '}' | ')' | ']' | '$')) || self.state_ahead(k)
    }

    /// A physical state in brackets: `Na+(aq)`, `Cl-(s)`.
    fn state_ahead(&self, k: usize) -> bool {
        ["(aq)", "(s)", "(l)", "(g)", "(sln)", "(cr)"]
            .iter()
            .any(|s| s.chars().enumerate().all(|(j, c)| self.peek(k + j) == Some(c)))
    }

    /// A balanced `{...}` starting at the current position, without braces.
    fn group(&mut self) -> Result<String, String> {
        let start = self.i + 1;
        let mut depth = 0usize;
        while let Some(ch) = self.peek(0) {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let s: String = self.c[start..self.i].iter().collect();
                        self.i += 1;
                        return Ok(s);
                    }
                }
                _ => {}
            }
            self.i += 1;
        }
        Err("unbalanced braces in \\ce".into())
    }

    /// `[...]` after an arrow, or `None`.
    fn bracket(&mut self) -> Result<Option<String>, String> {
        if self.peek(0) != Some('[') {
            return Ok(None);
        }
        let start = self.i + 1;
        let mut depth = 0usize;
        while let Some(ch) = self.peek(0) {
            match ch {
                '[' | '{' => depth += 1,
                ']' | '}' => {
                    depth -= 1;
                    if depth == 0 && ch == ']' {
                        let s: String = self.c[start..self.i].iter().collect();
                        self.i += 1;
                        return Ok(Some(s));
                    }
                }
                _ => {}
            }
            self.i += 1;
        }
        Err("unclosed `[` after an arrow in \\ce".into())
    }

    fn run(&mut self) -> Result<(), String> {
        while let Some(ch) = self.peek(0) {
            if let Some((tok, cmd)) = ARROWS.iter().find(|(t, _)| self.at(t)) {
                self.i += tok.chars().count();
                let above = self.bracket()?.map(|s| ce_to_tex(&s)).transpose()?.unwrap_or_default();
                let below = self.bracket()?.map(|s| ce_to_tex(&s)).transpose()?;
                // mhchem.sty: `{}\mathrel{arrow}{}`, the arrow at least 2 em
                // long (the parser's `\cex...` forms of the amsmath arrows).
                self.out.push_str(" {}\\ce");
                self.out.push_str(&cmd[1..]);
                if let Some(b) = below {
                    self.out.push('[');
                    self.out.push_str(&b);
                    self.out.push(']');
                }
                self.out.push('{');
                self.out.push_str(&above);
                self.out.push_str("}{} ");
                self.in_formula = false;
                continue;
            }
            match ch {
                ' ' => {
                    self.i += 1;
                    self.in_formula = false;
                    // ` ^` is a gas, ` v` a precipitate.
                    if self.peek(0) == Some('^') && self.boundary(1) {
                        self.i += 1;
                        self.out.push_str(r"{}\mathop{\uparrow}{} ");
                    } else if self.peek(0) == Some('v') && self.boundary(1) {
                        self.i += 1;
                        self.out.push_str(r"{}\mathop{\downarrow}{} ");
                    } else {
                        self.out.push(' ');
                    }
                }
                '$' => {
                    let end = self.c[self.i + 1..].iter().position(|&c| c == '$').ok_or("unclosed `$` in \\ce")?;
                    let math: String = self.c[self.i + 1..self.i + 1 + end].iter().collect();
                    self.out.push('{');
                    self.out.push_str(&math);
                    self.out.push('}');
                    self.i += end + 2;
                    self.in_formula = false;
                }
                '\\' => {
                    // A TeX command, passed through with its braced arguments.
                    self.out.push('\\');
                    self.i += 1;
                    while let Some(c) = self.peek(0) {
                        if c.is_ascii_alphabetic() {
                            self.out.push(c);
                            self.i += 1;
                        } else {
                            break;
                        }
                    }
                    if self.out.ends_with('\\') {
                        if let Some(c) = self.peek(0) {
                            self.out.push(c);
                            self.i += 1;
                        }
                    }
                    while self.peek(0) == Some('{') {
                        let g = self.group()?;
                        self.out.push('{');
                        self.out.push_str(&g);
                        self.out.push('}');
                    }
                    self.out.push(' ');
                    self.in_formula = false;
                }
                '{' => {
                    let g = self.group()?;
                    self.out.push('{');
                    self.out.push_str(&ce_to_tex(&g)?);
                    self.out.push('}');
                    self.in_formula = true;
                }
                'A'..='Z' => {
                    let mut el = String::from(ch);
                    self.i += 1;
                    while let Some(c @ 'a'..='z') = self.peek(0) {
                        el.push(c);
                        self.i += 1;
                    }
                    self.out.push_str(r"\mathrm{");
                    self.out.push_str(&el);
                    self.out.push('}');
                    self.in_formula = true;
                }
                'a'..='z' => {
                    let mut word = String::new();
                    while let Some(c @ 'a'..='z') = self.peek(0) {
                        word.push(c);
                        self.i += 1;
                    }
                    if word.chars().count() == 1 {
                        // A particle or a count: `e-`, `n`. mhchem sets them
                        // upright like element symbols, and an electron takes
                        // a charge like an ion does.
                        self.out.push_str(r"\mathrm{");
                        self.out.push_str(&word);
                        self.out.push('}');
                        self.in_formula = true;
                    } else {
                        self.out.push_str(r"\mathrm{");
                        self.out.push_str(&word);
                        self.out.push('}');
                        self.in_formula = false;
                    }
                }
                '0'..='9' => {
                    let mut num = String::new();
                    while let Some(c) = self.peek(0) {
                        if c.is_ascii_digit() || (c == '.' && self.peek(1).is_some_and(|d| d.is_ascii_digit())) {
                            num.push(c);
                            self.i += 1;
                        } else {
                            break;
                        }
                    }
                    if self.in_formula {
                        self.out.push_str(&sub(&num));
                    } else if self.peek(0) == Some('/') && self.peek(1).is_some_and(|d| d.is_ascii_digit()) {
                        // A fractional coefficient, `1/2 O2`.
                        self.i += 1;
                        let mut den = String::new();
                        while let Some(d @ '0'..='9') = self.peek(0) {
                            den.push(d);
                            self.i += 1;
                        }
                        self.out.push_str(&format!(r"\tfrac{{{num}}}{{{den}}}"));
                    } else {
                        self.out.push_str(&num);
                        // mhchem separates a coefficient from its formula.
                        if self
                            .peek(0)
                            .is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '(' | '[' | '^' | '_'))
                        {
                            self.out.push_str(r"\,");
                        }
                    }
                }
                '(' | '[' => {
                    self.out.push(ch);
                    self.i += 1;
                    self.in_formula = false;
                }
                ')' | ']' => {
                    self.out.push(ch);
                    self.i += 1;
                    self.in_formula = true;
                }
                '^' | '_' => {
                    self.i += 1;
                    let prescript = !self.in_formula && !self.out.ends_with('}');
                    let body = self.script_body()?;
                    // Mass and atomic number before a symbol (`^{227}_{90}Th`):
                    // mhchem right-aligns them, so the shorter is padded on the left.
                    let other = if ch == '^' { '_' } else { '^' };
                    if prescript && self.peek(0) == Some(other) {
                        self.i += 1;
                        let second = self.script_body()?;
                        let (above, below) = if ch == '^' { (body, second) } else { (second, body) };
                        self.out.push_str(&prescripts(&above, &below));
                        continue;
                    }
                    if prescript {
                        let (above, below) = if ch == '^' { (body.as_str(), "") } else { ("", body.as_str()) };
                        self.out.push_str(&prescripts(above, below));
                    } else if ch == '^' {
                        self.out.push_str(&sup(&body));
                    } else {
                        self.out.push_str(&sub(&body));
                    }
                }
                '+' | '-' if self.in_formula && self.charge_ahead() => {
                    let mut s = String::new();
                    while let Some(c @ ('+' | '-')) = self.peek(0) {
                        s.push(c);
                        self.i += 1;
                    }
                    self.out.push_str(&sup(&s));
                }
                '-' if self.in_formula => {
                    self.i += 1;
                    self.out.push_str("{-}");
                    self.in_formula = false;
                }
                '=' if self.in_formula => {
                    self.i += 1;
                    self.out.push_str("{=}");
                    self.in_formula = false;
                }
                '#' => {
                    self.i += 1;
                    self.out.push_str(r"{\equiv}");
                    self.in_formula = false;
                }
                '*' | '·' => {
                    self.i += 1;
                    self.out.push_str(r"\cdot ");
                    self.in_formula = false;
                }
                '.' if self.in_formula => {
                    self.i += 1;
                    self.out.push_str(r"\cdot ");
                    self.in_formula = false;
                }
                '%' | '&' => {
                    self.i += 1;
                    self.out.push('\\');
                    self.out.push(ch);
                }
                _ => {
                    self.out.push(ch);
                    self.i += 1;
                    self.in_formula = false;
                }
            }
        }
        Ok(())
    }

    /// What follows `^` or `_`: a braced group or a run of digits, letters and signs.
    fn script_body(&mut self) -> Result<String, String> {
        if self.peek(0) == Some('{') {
            return self.group();
        }
        let mut s = String::new();
        while let Some(c) = self.peek(0) {
            if c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.' {
                s.push(c);
                self.i += 1;
            } else {
                break;
            }
        }
        Ok(s)
    }

    /// A run of `+`/`-` that ends the formula is a charge (`Na+`, `OH-`,
    /// `Fe(CN)6^3-` is written with `^`); one followed by more formula is a bond.
    fn charge_ahead(&self) -> bool {
        let mut k = 0;
        while matches!(self.peek(k), Some('+' | '-')) {
            k += 1;
        }
        self.boundary(k)
    }
}

/// Script contents: a charge's sign is a superscript symbol, not an
/// operator, and Roman numerals (oxidation states) are upright.
/// A script's content. mhchem sets a formula in upright text, so its
/// scripts are text too: `\mathrm` digits are the text font's, not the math
/// font's script-size shapes.
fn script(s: &str) -> String {
    format!(r"\mathrm{{{}}}", s.replace('-', "{-}").replace('+', "{+}"))
}

// The three forms below are mhchem.sty's `coreFive` output in its default
// layout (staggered-flat): every script hangs on an invisible X, so scripts
// sit at one height whatever letter they follow.

/// A superscript after a formula: a charge, an oxidation state.
fn sup(s: &str) -> String {
    format!(r"{{\vphantom{{\mathrm{{X}}}}}}^{{{}}}", script(s))
}

/// A subscript after a formula, its top smashed so it cannot push away
/// from the baseline.
fn sub(s: &str) -> String {
    format!(r"{{\vphantom{{\mathrm{{X}}}}}}_{{\smash[t]{{{}}}}}", script(s))
}

/// Mass and atomic number before a symbol (`^{227}_{90}Th`): set to the
/// left of where they stand, so they are right-aligned, in a space as wide
/// as the wider one; the kern takes back the script space after them.
fn prescripts(above: &str, below: &str) -> String {
    let (a, b) = (script(above), script(below));
    let mut out = String::from(r"\hphantom{{}");
    if !above.is_empty() {
        out.push_str(&format!("^{{{a}}}"));
    }
    if !below.is_empty() {
        out.push_str(&format!("_{{{b}}}"));
    }
    out.push_str(r"}{\vphantom{\mathrm{X}}}");
    out.push_str(&format!(
        r"^{{\llap{{\vphantom{{\smash[t]{{\mathrm{{2}}}}}}{}}}}}",
        if above.is_empty() { String::new() } else { a }
    ));
    if !below.is_empty() {
        out.push_str(&format!(r"_{{\llap{{\vphantom{{\mathrm{{2}}}}\smash[t]{{{b}}}}}}}"));
    }
    out.push_str(r"\mkern-1mu ");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_and_charges() {
        // Scripts hang on an invisible X, as mhchem.sty sets them.
        let (s2, s3, s4) = (sub("2"), sub("3"), sub("4"));
        assert_eq!(s2, r"{\vphantom{\mathrm{X}}}_{\smash[t]{\mathrm{2}}}");
        assert_eq!(sup("2-"), r"{\vphantom{\mathrm{X}}}^{\mathrm{2{-}}}");
        assert_eq!(ce_to_tex("H2O").unwrap(), format!(r"\mathrm{{H}}{s2}\mathrm{{O}}"));
        assert_eq!(ce_to_tex("Na+").unwrap(), format!(r"\mathrm{{Na}}{}", sup("+")));
        assert_eq!(ce_to_tex("SO4^2-").unwrap(), format!(r"\mathrm{{S}}\mathrm{{O}}{s4}{}", sup("2-")));
        assert!(ce_to_tex("Na+(aq)").unwrap().starts_with(&format!(r"\mathrm{{Na}}{}(", sup("+"))));
        assert_eq!(ce_to_tex("NO3-").unwrap(), format!(r"\mathrm{{N}}\mathrm{{O}}{s3}{}", sup("-")));
        assert_eq!(ce_to_tex("Fe^{III}").unwrap(), format!(r"\mathrm{{Fe}}{}", sup("III")));
        assert_eq!(
            ce_to_tex("(NH4)2SO4").unwrap(),
            format!(r"(\mathrm{{N}}\mathrm{{H}}{s4}){s2}\mathrm{{S}}\mathrm{{O}}{s4}")
        );
        assert_eq!(ce_to_tex("2H2").unwrap(), format!(r"2\,\mathrm{{H}}{s2}"));
        assert_eq!(ce_to_tex("5e-").unwrap(), format!(r"5\,\mathrm{{e}}{}", sup("-")));
        assert_eq!(
            ce_to_tex("CuSO4*5H2O").unwrap(),
            format!(r"\mathrm{{Cu}}\mathrm{{S}}\mathrm{{O}}{s4}\cdot 5\,\mathrm{{H}}{s2}\mathrm{{O}}")
        );
    }

    #[test]
    fn arrows_and_marks() {
        assert!(ce_to_tex("A -> B").unwrap().contains(r"{}\cexrightarrow{}{}"));
        assert!(ce_to_tex("A <=> B").unwrap().contains(r"{}\cexrightleftharpoons{}{}"));
        assert!(ce_to_tex("A ->[\\Delta][cat] B")
            .unwrap()
            .contains(r"\cexrightarrow[\mathrm{cat}]{\Delta }{}"));
        assert!(ce_to_tex("CO2 ^").unwrap().contains(r"\uparrow"));
        assert!(ce_to_tex("AgCl v").unwrap().contains(r"\downarrow"));
        assert!(ce_to_tex("CH3-CH3").unwrap().contains("{-}"));
        // Prescripts: a phantom as wide as both, then each set leftwards.
        let th = ce_to_tex("^{227}_{90}Th").unwrap();
        assert!(
            th.starts_with(r"\hphantom{{}^{\mathrm{227}}_{\mathrm{90}}}{\vphantom{\mathrm{X}}}^{\llap{"),
            "{th}"
        );
        assert!(th.ends_with(r"\mkern-1mu \mathrm{Th}"), "{th}");
    }

    #[test]
    fn units() {
        assert_eq!(pu_to_tex("123 kJ/mol"), r"123\,\mathrm{kJ}/\mathrm{mol}");
        assert_eq!(pu_to_tex("1.2e3 m.s^-2"), r"1.2\times 10^{3}\,\mathrm{m}\,\mathrm{s}^{-2}");
    }
}
