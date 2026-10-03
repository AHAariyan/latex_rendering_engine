//! Numbers and units from siunitx: `\num`, `\unit`/`\si`, `\qty`/`\SI`,
//! `\ang`, ranges, lists and products, with siunitx's default (v3) output:
//!
//! - digits grouped in threes by thin spaces from five digits up;
//! - `e`/`E` exponents as `×10ⁿ`, `+-` as `±`, a leading `.` given its `0`;
//! - units upright, separated by thin spaces, `\per` as a negative power;
//! - a literal unit (`m/s^2`, `kg.m`) kept as written, `.` and `~` as thin spaces.
//!
//! Options in `[...]` are read and ignored.

use crate::error::Result;
use crate::lexer::Lexer;

/// Translates a siunitx command whose name has just been read, or `None`.
pub(super) fn translate(name: &str, lx: &mut Lexer<'_>) -> Result<Option<String>> {
    Ok(Some(match name {
        "num" => {
            lx.raw_optional()?;
            number(lx.raw_group()?)
        }
        "si" | "unit" => {
            lx.raw_optional()?;
            spoken_unit(lx.raw_group()?, true)
        }
        "SI" | "qty" => {
            lx.raw_optional()?;
            let n = lx.raw_group()?;
            // siunitx 2's pre-unit: `\SI{10}[\$]{}`.
            let pre = lx.raw_optional()?.map(|p| format!("{p}\\,")).unwrap_or_default();
            let u = lx.raw_group()?;
            format!("{pre}{}", quantity(n, u))
        }
        "ang" => {
            lx.raw_optional()?;
            angle(lx.raw_group()?)
        }
        "numrange" | "SIrange" | "qtyrange" => {
            lx.raw_optional()?;
            let a = lx.raw_group()?;
            let b = lx.raw_group()?;
            if name == "numrange" {
                format!(r"{}\text{{ to }}{}", number(a), number(b))
            } else {
                let u = lx.raw_group()?;
                format!(r"{}\text{{ to }}{}", quantity(a, u), quantity(b, u))
            }
        }
        "numlist" | "SIlist" | "qtylist" => {
            lx.raw_optional()?;
            let items = lx.raw_group()?;
            let u = if name == "numlist" { None } else { Some(lx.raw_group()?) };
            let items: Vec<String> = items.split(';').map(|n| u.map_or_else(|| number(n), |u| quantity(n, u))).collect();
            list(&items)
        }
        "numproduct" | "qtyproduct" => {
            lx.raw_optional()?;
            let items = lx.raw_group()?;
            let u = if name == "numproduct" { None } else { Some(lx.raw_group()?) };
            items
                .split('x')
                .map(|n| u.map_or_else(|| number(n), |u| quantity(n, u)))
                .collect::<Vec<_>>()
                .join(r"\times ")
        }
        _ => return Ok(None),
    }))
}

/// `number` and `unit` with siunitx's spacing between them.
pub(super) fn quantity(n: &str, u: &str) -> String {
    let tex = unit(u);
    if tex.is_empty() {
        return number(n);
    }
    // Angles sit against their number: 30°, not 30 °; but 25 °C.
    let angle = (tex.starts_with(DEGREE) && !tex.starts_with(CELSIUS)) || tex.starts_with("{}'");
    let sep = if angle { "" } else { r"\," };
    // One meter, two meters.
    let plural = n.trim().trim_start_matches('+') != "1";
    format!("{}{sep}{}", number(n), spoken_unit(u, plural))
}

/// The unit's TeX, marked with how it is read aloud when that is known.
fn spoken_unit(src: &str, plural: bool) -> String {
    let tex = unit(src);
    match unit_speech(src, plural) {
        Some(speech) if !tex.is_empty() => format!(r"\spokenas{{{speech}}}{{{tex}}}"),
        _ => tex,
    }
}

/// "kilometers per hour" for `\kilo\meter\per\hour` or `km/h`, or `None`
/// for a unit this cannot name.
fn unit_speech(src: &str, plural: bool) -> Option<String> {
    // (name, power, per)
    let mut parts: Vec<(String, u32, bool)> = vec![];
    let src = src.trim();
    if src.contains('\\') {
        let mut lx = UnitLexer { s: src, i: 0 };
        let (mut prefix, mut pre_power, mut per) = (String::new(), None::<u32>, false);
        while let Some(tok) = lx.next() {
            let UnitTok::Cmd(c) = tok else { return None };
            if PREFIXES.iter().any(|(n, _)| *n == c) {
                prefix.push_str(c);
            } else if UNITS.iter().any(|(n, _)| *n == c) {
                parts.push((format!("{prefix}{}", unit_name(c)), pre_power.take().unwrap_or(1), per));
                prefix.clear();
                per = false;
            } else {
                match c {
                    "per" => per = true,
                    "square" => pre_power = Some(2),
                    "cubic" => pre_power = Some(3),
                    "raiseto" => pre_power = Some(lx.group().trim().parse().ok()?),
                    "squared" | "cubed" | "tothe" => {
                        let p = match c {
                            "squared" => 2,
                            "cubed" => 3,
                            _ => lx.group().trim().parse().ok()?,
                        };
                        parts.last_mut()?.1 = p;
                    }
                    _ => return None,
                }
            }
        }
    } else {
        let chars: Vec<char> = src.chars().collect();
        let (mut i, mut per) = (0, false);
        while i < chars.len() {
            let c = chars[i];
            if c.is_alphabetic() {
                let start = i;
                while i < chars.len() && chars[i].is_alphabetic() {
                    i += 1;
                }
                let sym: String = chars[start..i].iter().collect();
                parts.push((symbol_name(&sym)?, 1, per));
            } else if c == '%' {
                parts.push(("percent".into(), 1, per));
                i += 1;
            } else if c == '/' {
                per = true;
                i += 1;
            } else if matches!(c, '.' | '~' | ' ') {
                i += 1;
            } else if c == '^' {
                let rest: String = chars[i + 1..].iter().collect();
                let digits: String = rest
                    .trim_start_matches('{')
                    .chars()
                    .take_while(|c| *c == '-' || c.is_ascii_digit())
                    .collect();
                let n: i32 = digits.parse().ok()?;
                let last = parts.last_mut()?;
                last.1 = n.unsigned_abs();
                if n < 0 {
                    last.2 = !last.2;
                }
                i += 1 + rest
                    .find(|c: char| !(c == '{' || c == '}' || c == '-' || c.is_ascii_digit()))
                    .unwrap_or(rest.len());
            } else {
                return None;
            }
        }
    }
    if parts.is_empty() {
        return None;
    }
    // The last unit before "per" is the plural one: "newton meters per second".
    let plural_at = parts.iter().rposition(|p| !p.2).filter(|_| plural);
    let mut words = vec![];
    for (i, (name, power, per)) in parts.iter().enumerate() {
        if *per {
            words.push("per".to_string());
        }
        words.push(if Some(i) == plural_at { plural_of(name) } else { name.clone() });
        match power {
            1 => {}
            2 => words.push("squared".into()),
            3 => words.push("cubed".into()),
            n => words.push(format!("to the {n}")),
        }
    }
    Some(words.join(" "))
}

/// The spoken name of a unit command: `\degreeCelsius` is "degree Celsius".
fn unit_name(cmd: &str) -> &str {
    match cmd {
        "degreeCelsius" => "degree Celsius",
        "astronomicalunit" => "astronomical unit",
        "atomicmassunit" => "atomic mass unit",
        "nauticalmile" => "nautical mile",
        "mmHg" => "millimeter of mercury",
        "clight" => "speed of light",
        "electronmass" => "electron mass",
        "planckbar" => "reduced Planck constant",
        "elementarycharge" => "elementary charge",
        "liter" | "litre" => "liter",
        c => c,
    }
}

/// The unit a literal symbol stands for: `km` is "kilometer".
fn symbol_name(sym: &str) -> Option<String> {
    let named = |s: &str| {
        UNITS
            .iter()
            .find(|(_, u)| *u == s && !u.starts_with(['{', '\\']))
            .map(|(c, _)| unit_name(c).to_string())
    };
    if let Some(n) = named(sym) {
        return Some(n);
    }
    PREFIXES
        .iter()
        .find_map(|(p, ps)| sym.strip_prefix(ps).and_then(named).map(|n| format!("{p}{n}")))
}

fn plural_of(name: &str) -> String {
    if let Some(rest) = name.strip_suffix(" Celsius") {
        return format!("{rest}s Celsius");
    }
    if let Some((head, tail)) = name.split_once(" of ") {
        return format!("{head}s of {tail}");
    }
    match name {
        n if n.ends_with("hertz") || n.ends_with("siemens") || n.ends_with("lux") || n.ends_with("percent") => n.into(),
        n if n.ends_with("henry") => format!("{}ies", &n[..n.len() - 1]),
        "speed of light" | "reduced Planck constant" => name.into(),
        n => format!("{n}s"),
    }
}

const DEGREE: &str = r"{}^{\circ}";
const CELSIUS: &str = r"{}^{\circ}\mathrm{C}";

fn list(items: &[String]) -> String {
    match items {
        [] => String::new(),
        [a] => a.clone(),
        [a, b] => format!(r"{a}\text{{ and }}{b}"),
        [init @ .., last] => format!(r"{}\text{{ and }}{last}", init.join(r",\ ")),
    }
}

/// A number: grouped digits, exponent, uncertainty, sign.
fn number(src: &str) -> String {
    let s: String = src.chars().filter(|c| !c.is_whitespace()).collect();
    if s.is_empty() {
        return String::new();
    }
    // Anything with TeX in it is the author's own formatting.
    if s.contains('\\') && !s.contains(r"\pm") {
        return s;
    }
    let (mantissa, exponent) = match s.find(['e', 'E', 'd', 'D']) {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (&s[..], None),
    };
    let mut out = String::new();
    if !mantissa.is_empty() {
        out.push_str(&with_uncertainty(mantissa));
    }
    if let Some(e) = exponent {
        let e = e.strip_prefix('+').unwrap_or(e);
        if !out.is_empty() {
            out.push_str(r"\times ");
        }
        out.push_str(&format!("10^{{{}}}", sign_and_digits(e)));
    }
    out
}

/// Uncertainties in siunitx's compact form: `1.25+-0.03` and `1.25(3)`
/// are both 1.25(3).
fn with_uncertainty(s: &str) -> String {
    let s = s.replace(r"\pm", "+-");
    match s.split_once("+-") {
        Some((v, u)) => match compact(v, u) {
            Some((v, u)) => format!("{}({u})", sign_and_digits(&v)),
            None => format!(r"{}\pm {}", sign_and_digits(v), sign_and_digits(u)),
        },
        None => match s.split_once('(') {
            Some((v, rest)) => format!("{}({rest}", sign_and_digits(v)),
            None => sign_and_digits(&s),
        },
    }
}

/// `1.25`, `0.03` → `1.25`, `3`: the uncertainty in units of the value's
/// last digit, the value padded with zeros if the uncertainty is finer.
fn compact(v: &str, u: &str) -> Option<(String, String)> {
    let decimals = |x: &str| x.find(['.', ',']).map_or(0, |i| x.len() - i - 1);
    let digits = |x: &str| x.chars().all(|c| c.is_ascii_digit() || c == '.' || c == ',');
    let body = v.trim_start_matches(['-', '+']);
    if !digits(body) || !digits(u) || u.is_empty() {
        return None;
    }
    let (dv, du) = (decimals(v), decimals(u));
    let mut v = v.to_string();
    if du > dv {
        if dv == 0 {
            v.push('.');
        }
        v.push_str(&"0".repeat(du - dv));
    }
    let mut u: String = u.chars().filter(char::is_ascii_digit).collect();
    u.push_str(&"0".repeat(dv.saturating_sub(du)));
    let u = u.trim_start_matches('0');
    Some((v, if u.is_empty() { "0".into() } else { u.into() }))
}

fn sign_and_digits(s: &str) -> String {
    let (sign, rest) = match s.chars().next() {
        Some('-') => ("-", &s[1..]),
        Some('+') => ("", &s[1..]),
        _ => ("", s),
    };
    let (int, frac) = match rest.find(['.', ',']) {
        Some(i) => (&rest[..i], Some(&rest[i + 1..])),
        None => (rest, None),
    };
    let int = if int.is_empty() && frac.is_some() { "0" } else { int };
    let mut out = String::from(sign);
    out.push_str(&group(int, true));
    if let Some(f) = frac {
        out.push('.');
        out.push_str(&group(f, false));
    }
    out
}

/// Thin spaces between groups of three, counted from the decimal point,
/// once there are five digits or more.
fn group(digits: &str, from_right: bool) -> String {
    let n = digits.chars().count();
    if n < 5 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return digits.to_string();
    }
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        let k = if from_right { n - i } else { i };
        if i > 0 && k % 3 == 0 {
            out.push_str(r"\,");
        }
        out.push(c);
    }
    out
}

/// A unit, either `\kilo\meter\per\second` or a literal `km/s`.
fn unit(src: &str) -> String {
    let src = src.trim();
    if !src.contains('\\') {
        return literal_unit(src);
    }
    let mut lx = UnitLexer { s: src, i: 0 };
    let mut parts: Vec<Part> = vec![];
    let (mut prefix, mut pre_power, mut per) = (String::new(), None::<String>, false);
    while let Some(tok) = lx.next() {
        match tok {
            UnitTok::Cmd(c) => {
                if let Some(p) = PREFIXES.iter().find(|(n, _)| *n == c) {
                    prefix.push_str(p.1);
                } else if let Some(u) = UNITS.iter().find(|(n, _)| *n == c) {
                    parts.push(Part {
                        body: upright(&prefix, u.1),
                        power: pre_power.take(),
                        per: std::mem::take(&mut per),
                    });
                    prefix.clear();
                } else {
                    match c {
                        "per" => per = true,
                        "square" => pre_power = Some("2".into()),
                        "cubic" => pre_power = Some("3".into()),
                        "raiseto" => pre_power = Some(lx.group().to_string()),
                        "squared" | "cubed" | "tothe" => {
                            let p = match c {
                                "squared" => "2".to_string(),
                                "cubed" => "3".to_string(),
                                _ => lx.group().to_string(),
                            };
                            if let Some(last) = parts.last_mut() {
                                last.power = Some(p);
                            }
                        }
                        "of" => {
                            let q = lx.group().to_string();
                            if let Some(last) = parts.last_mut() {
                                last.body = format!("{}_{{\\mathrm{{{q}}}}}", last.body);
                            }
                        }
                        "highlight" => {
                            lx.group();
                        }
                        other => parts.push(Part {
                            body: format!("\\{other} "),
                            power: None,
                            per: false,
                        }),
                    }
                }
            }
            UnitTok::Text(t) => parts.push(Part {
                body: literal_unit(t),
                power: None,
                per: false,
            }),
        }
    }
    parts
        .iter()
        .map(|p| {
            let power = match (&p.power, p.per) {
                (None, false) => String::new(),
                (Some(n), false) => format!("^{{{n}}}"),
                (None, true) => "^{-1}".into(),
                (Some(n), true) => format!("^{{-{n}}}"),
            };
            format!("{}{power}", p.body)
        })
        .collect::<Vec<_>>()
        .join(r"\,")
}

struct Part {
    body: String,
    power: Option<String>,
    per: bool,
}

/// Prefix and symbol as one upright group: `\mathrm{km}`. Symbols that are
/// TeX already (`{}^{\circ}`) are kept as they are.
fn upright(prefix: &str, sym: &str) -> String {
    if sym.starts_with('{') || sym.starts_with('\\') {
        if prefix.is_empty() {
            sym.to_string()
        } else {
            format!(r"\mathrm{{{prefix}}}{sym}")
        }
    } else {
        format!(r"\mathrm{{{prefix}{sym}}}")
    }
}

/// `m/s^2`, `kg.m`, `N~m`: letters upright, `.` and `~` thin spaces.
fn literal_unit(s: &str) -> String {
    let mut out = String::new();
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if !run.is_empty() {
            out.push_str(&format!(r"\mathrm{{{run}}}"));
            run.clear();
        }
    };
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            c if c.is_alphabetic() || c == '%' || c == 'Ω' || c == 'µ' || c == 'Å' => {
                if c == '%' {
                    run.push_str(r"\%");
                } else {
                    run.push(c);
                }
            }
            '.' | '~' | ' ' => {
                flush(&mut run, &mut out);
                if !out.is_empty() && !out.ends_with(r"\,") {
                    out.push_str(r"\,");
                }
            }
            '^' | '_' => {
                flush(&mut run, &mut out);
                out.push(c);
                // The script: a group, or one token (a sign and digits count as one).
                let mut script = String::new();
                if chars.peek() == Some(&'{') {
                    let mut depth = 0;
                    for c in chars.by_ref() {
                        script.push(c);
                        match c {
                            '{' => depth += 1,
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    out.push_str(&script);
                } else {
                    if chars.peek() == Some(&'-') {
                        script.push(chars.next().unwrap());
                    }
                    while let Some(&d) = chars.peek() {
                        if d.is_ascii_digit() {
                            script.push(d);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    if script.is_empty() {
                        script.extend(chars.next());
                    }
                    out.push_str(&format!("{{{script}}}"));
                }
            }
            c => {
                flush(&mut run, &mut out);
                out.push(c);
            }
        }
    }
    flush(&mut run, &mut out);
    out.trim_end_matches(r"\,").to_string()
}

struct UnitLexer<'a> {
    s: &'a str,
    i: usize,
}

enum UnitTok<'a> {
    Cmd(&'a str),
    Text(&'a str),
}

impl<'a> UnitLexer<'a> {
    fn next(&mut self) -> Option<UnitTok<'a>> {
        let rest = &self.s[self.i..];
        let trimmed = rest.trim_start();
        self.i += rest.len() - trimmed.len();
        let rest = trimmed;
        if rest.is_empty() {
            return None;
        }
        if let Some(r) = rest.strip_prefix('\\') {
            let n = r.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(r.len()).max(1.min(r.len()));
            self.i += 1 + n;
            return Some(UnitTok::Cmd(&r[..n]));
        }
        let n = rest.find('\\').unwrap_or(rest.len());
        self.i += n;
        Some(UnitTok::Text(rest[..n].trim()))
    }

    /// The `{...}` argument of `\tothe` and the like.
    fn group(&mut self) -> &'a str {
        let rest = self.s[self.i..].trim_start();
        self.i = self.s.len() - rest.len();
        if let Some(r) = rest.strip_prefix('{') {
            let mut depth = 1;
            for (j, c) in r.char_indices() {
                match c {
                    '{' => depth += 1,
                    '}' => {
                        depth -= 1;
                        if depth == 0 {
                            self.i += 2 + j;
                            return &r[..j];
                        }
                    }
                    _ => {}
                }
            }
            self.i = self.s.len();
            r
        } else {
            let n = rest.chars().next().map_or(0, char::len_utf8);
            self.i += n;
            &rest[..n]
        }
    }
}

/// `\ang{1;2;3}`: degrees, minutes, seconds.
fn angle(src: &str) -> String {
    let marks = [DEGREE, "{}'", "{}''"];
    src.split(';')
        .zip(marks)
        .filter(|(v, _)| !v.trim().is_empty())
        .map(|(v, m)| format!("{}{m}", number(v)))
        .collect()
}

const PREFIXES: &[(&str, &str)] = &[
    ("quecto", "q"),
    ("ronto", "r"),
    ("yocto", "y"),
    ("zepto", "z"),
    ("atto", "a"),
    ("femto", "f"),
    ("pico", "p"),
    ("nano", "n"),
    ("micro", "µ"),
    ("milli", "m"),
    ("centi", "c"),
    ("deci", "d"),
    ("deca", "da"),
    ("deka", "da"),
    ("hecto", "h"),
    ("kilo", "k"),
    ("mega", "M"),
    ("giga", "G"),
    ("tera", "T"),
    ("peta", "P"),
    ("exa", "E"),
    ("zetta", "Z"),
    ("yotta", "Y"),
    ("ronna", "R"),
    ("quetta", "Q"),
    ("kibi", "Ki"),
    ("mebi", "Mi"),
    ("gibi", "Gi"),
];

const UNITS: &[(&str, &str)] = &[
    ("ampere", "A"),
    ("candela", "cd"),
    ("kelvin", "K"),
    ("kilogram", "kg"),
    ("gram", "g"),
    ("meter", "m"),
    ("metre", "m"),
    ("mole", "mol"),
    ("second", "s"),
    ("becquerel", "Bq"),
    ("degreeCelsius", CELSIUS),
    ("coulomb", "C"),
    ("farad", "F"),
    ("gray", "Gy"),
    ("hertz", "Hz"),
    ("henry", "H"),
    ("joule", "J"),
    ("katal", "kat"),
    ("lumen", "lm"),
    ("lux", "lx"),
    ("newton", "N"),
    ("ohm", "Ω"),
    ("pascal", "Pa"),
    ("radian", "rad"),
    ("siemens", "S"),
    ("sievert", "Sv"),
    ("steradian", "sr"),
    ("tesla", "T"),
    ("volt", "V"),
    ("watt", "W"),
    ("weber", "Wb"),
    ("astronomicalunit", "au"),
    ("bel", "B"),
    ("dalton", "Da"),
    ("day", "d"),
    ("decibel", "dB"),
    ("degree", DEGREE),
    ("electronvolt", "eV"),
    ("hectare", "ha"),
    ("hour", "h"),
    ("litre", "L"),
    ("liter", "L"),
    ("arcminute", "{}'"),
    ("arcsecond", "{}''"),
    ("minute", "min"),
    ("neper", "Np"),
    ("tonne", "t"),
    ("percent", r"\%"),
    ("angstrom", "Å"),
    ("bar", "bar"),
    ("barn", "b"),
    ("knot", "kn"),
    ("mmHg", "mmHg"),
    ("nauticalmile", "M"),
    ("byte", "B"),
    ("bit", "bit"),
    ("atomicmassunit", "u"),
    ("clight", "c"),
    ("electronmass", r"m_{\mathrm{e}}"),
    ("planckbar", r"\hbar"),
    ("elementarycharge", "e"),
    ("bohr", r"a_{0}"),
    ("hartree", r"E_{\mathrm{h}}"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(number("12345.678"), r"12\,345.678");
        assert_eq!(number("1234"), "1234");
        assert_eq!(number("0.123456"), r"0.123\,456");
        assert_eq!(number("6.022e23"), r"6.022\times 10^{23}");
        assert_eq!(number("-1.5E-3"), r"-1.5\times 10^{-3}");
        assert_eq!(number("e5"), "10^{5}");
        assert_eq!(number(".5"), "0.5");
        assert_eq!(number("1.25+-0.03"), "1.25(3)");
        assert_eq!(number("1.2+-0.03"), "1.20(3)");
        assert_eq!(number("12+-3"), "12(3)");
        assert_eq!(number("1.2+-0.3"), "1.2(3)");
        assert_eq!(number("1.23(4)"), "1.23(4)");
        assert_eq!(number("+7"), "7");
    }

    #[test]
    fn units() {
        assert_eq!(unit(r"\kilo\gram"), r"\mathrm{kg}");
        assert_eq!(unit(r"\meter\per\second\squared"), r"\mathrm{m}\,\mathrm{s}^{-2}");
        assert_eq!(unit(r"\per\square\meter"), r"\mathrm{m}^{-2}");
        assert_eq!(unit(r"\kilo\watt\hour"), r"\mathrm{kW}\,\mathrm{h}");
        assert_eq!(unit(r"\micro\ohm"), r"\mathrm{µΩ}");
        assert_eq!(unit(r"\meter\tothe{4}"), r"\mathrm{m}^{4}");
        assert_eq!(unit("m/s^2"), r"\mathrm{m}/\mathrm{s}^{2}");
        assert_eq!(unit("kg.m^2.s^-1"), r"\mathrm{kg}\,\mathrm{m}^{2}\,\mathrm{s}^{-1}");
        assert_eq!(unit("J mol^{-1}"), r"\mathrm{J}\,\mathrm{mol}^{-1}");
    }

    #[test]
    fn units_are_spoken() {
        let say = |u: &str| unit_speech(u, true).unwrap();
        assert_eq!(say(r"\meter\per\second\squared"), "meters per second squared");
        assert_eq!(say(r"\kilo\watt\hour"), "kilowatt hours");
        assert_eq!(say(r"\kilogram\per\cubic\meter"), "kilograms per meter cubed");
        assert_eq!(say(r"\newton\meter"), "newton meters");
        assert_eq!(say(r"\degreeCelsius"), "degrees Celsius");
        assert_eq!(say(r"\micro\farad"), "microfarads");
        assert_eq!(say(r"\hertz"), "hertz");
        assert_eq!(say(r"\per\second"), "per second");
        assert_eq!(say("m/s^2"), "meters per second squared");
        assert_eq!(say("km/h"), "kilometers per hour");
        assert_eq!(say("kg.m^2.s^-1"), "kilogram meters squared per second");
        assert_eq!(say("J mol^{-1}"), "joules per mole");
        assert_eq!(unit_speech("m", false).unwrap(), "meter");
        assert_eq!(unit_speech("xyz", true), None);
    }

    #[test]
    fn quantities() {
        assert_eq!(
            quantity("9.8", "m/s^2"),
            r"9.8\,\spokenas{meters per second squared}{\mathrm{m}/\mathrm{s}^{2}}"
        );
        assert_eq!(quantity("30", r"\degree"), r"30\spokenas{degrees}{{}^{\circ}}");
        assert_eq!(
            quantity("25", r"\degreeCelsius"),
            r"25\,\spokenas{degrees Celsius}{{}^{\circ}\mathrm{C}}"
        );
        assert_eq!(angle("1;2;3"), r"1{}^{\circ}2{}'3{}''");
        assert_eq!(list(&["1".into(), "2".into(), "3".into()]), r"1,\ 2\text{ and }3");
    }

    #[test]
    fn every_command_parses() {
        for tex in [
            r"\num{1.5e-3}",
            r"\si{\kilo\meter\per\hour}",
            r"\unit{m/s}",
            r"\SI{9.81}{\meter\per\second\squared}",
            r"\SI[per-mode=symbol]{3}[\$]{}",
            r"\qty{1.2}{\kilo\joule\per\mole}",
            r"\ang{12;30;0}",
            r"\SIrange{1}{10}{\milli\liter}",
            r"\numlist{1;2;3}",
            r"\qtylist{1;2}{\meter}",
            r"\numproduct{2x3x4}",
            r"\qtyproduct{2x3}{\meter}",
            r"\qty{25}{\degreeCelsius}",
            r"\qty{50}{\percent}",
        ] {
            crate::parser::parse(tex).unwrap_or_else(|e| panic!("{tex}: {e}"));
        }
    }
}
