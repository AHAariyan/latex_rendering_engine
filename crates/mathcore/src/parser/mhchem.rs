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
        matches!(self.peek(k), None | Some(' ' | '}' | ')' | ']' | '$'))
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
                self.out.push(' ');
                self.out.push_str(cmd);
                // mhchem's arrows are long even without labels.
                let above = if above.is_empty() { r"\hphantom{MM}".to_string() } else { above };
                if let Some(b) = below {
                    self.out.push('[');
                    self.out.push_str(&b);
                    self.out.push(']');
                }
                self.out.push('{');
                self.out.push_str(&above);
                self.out.push_str("} ");
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
                        self.out.push_str(r"\uparrow ");
                    } else if self.peek(0) == Some('v') && self.boundary(1) {
                        self.i += 1;
                        self.out.push_str(r"\downarrow ");
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
                        self.out.push_str("_{");
                        self.out.push_str(&num);
                        self.out.push('}');
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
                        if self.peek(0).is_some_and(|c| c.is_ascii_alphabetic() || c == '(' || c == '[') {
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
                    // A prescript (`^{227}_{90}Th`) hangs off an empty base.
                    if !self.in_formula && !self.out.ends_with('}') {
                        self.out.push_str("{}");
                    } else if ch == '^' && self.in_formula {
                        self.stagger();
                    }
                    self.out.push(ch);
                    self.out.push('{');
                    if self.peek(0) == Some('{') {
                        let g = self.group()?;
                        self.out.push_str(&script(&g));
                    } else {
                        let mut s = String::new();
                        while let Some(c) = self.peek(0) {
                            if c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.' {
                                s.push(c);
                                self.i += 1;
                            } else {
                                break;
                            }
                        }
                        self.out.push_str(&script(&s));
                    }
                    self.out.push('}');
                }
                '+' | '-' if self.in_formula && self.charge_ahead() => {
                    let mut s = String::new();
                    while let Some(c @ ('+' | '-')) = self.peek(0) {
                        s.push(c);
                        self.i += 1;
                    }
                    self.stagger();
                    self.out.push_str("^{");
                    self.out.push_str(&script(&s));
                    self.out.push('}');
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

    /// A charge after a subscript sits to its right, as mhchem sets it
    /// (`SO4^2-` is SO₄²⁻ side by side, not stacked).
    fn stagger(&mut self) {
        if self.out.ends_with('}') && self.out.rfind("_{").is_some_and(|i| !self.out[i..].contains('^')) {
            self.out.push_str("{}");
        }
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
fn script(s: &str) -> String {
    if !s.is_empty() && s.chars().all(|c| matches!(c, 'I' | 'V' | 'X')) {
        return format!(r"\mathrm{{{s}}}");
    }
    s.replace('-', "{-}").replace('+', "{+}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formulas_and_charges() {
        assert_eq!(ce_to_tex("H2O").unwrap(), r"\mathrm{H}_{2}\mathrm{O}");
        assert_eq!(ce_to_tex("Na+").unwrap(), r"\mathrm{Na}^{{+}}");
        assert_eq!(ce_to_tex("SO4^2-").unwrap(), r"\mathrm{S}\mathrm{O}_{4}{}^{2{-}}");
        assert_eq!(ce_to_tex("NO3-").unwrap(), r"\mathrm{N}\mathrm{O}_{3}{}^{{-}}");
        assert_eq!(ce_to_tex("Fe^{III}").unwrap(), r"\mathrm{Fe}^{\mathrm{III}}");
        assert_eq!(
            ce_to_tex("(NH4)2SO4").unwrap(),
            r"(\mathrm{N}\mathrm{H}_{4})_{2}\mathrm{S}\mathrm{O}_{4}"
        );
        assert_eq!(ce_to_tex("2H2").unwrap(), r"2\,\mathrm{H}_{2}");
        assert_eq!(ce_to_tex("5e-").unwrap(), r"5\,\mathrm{e}^{{-}}");
        assert_eq!(
            ce_to_tex("CuSO4*5H2O").unwrap(),
            r"\mathrm{Cu}\mathrm{S}\mathrm{O}_{4}\cdot 5\,\mathrm{H}_{2}\mathrm{O}"
        );
    }

    #[test]
    fn arrows_and_marks() {
        assert!(ce_to_tex("A -> B").unwrap().contains(r"\xrightarrow{\hphantom{MM}}"));
        assert!(ce_to_tex("A <=> B").unwrap().contains(r"\xrightleftharpoons{\hphantom{MM}}"));
        assert!(ce_to_tex("A ->[\\Delta][cat] B")
            .unwrap()
            .contains(r"\xrightarrow[\mathrm{cat}]{\Delta }"));
        assert!(ce_to_tex("CO2 ^").unwrap().contains(r"\uparrow"));
        assert!(ce_to_tex("AgCl v").unwrap().contains(r"\downarrow"));
        assert!(ce_to_tex("CH3-CH3").unwrap().contains("{-}"));
        assert!(ce_to_tex("^{227}_{90}Th").unwrap().starts_with("{}^{227}"));
    }

    #[test]
    fn units() {
        assert_eq!(pu_to_tex("123 kJ/mol"), r"123\,\mathrm{kJ}/\mathrm{mol}");
        assert_eq!(pu_to_tex("1.2e3 m.s^-2"), r"1.2\times 10^{3}\,\mathrm{m}\,\mathrm{s}^{-2}");
    }
}
