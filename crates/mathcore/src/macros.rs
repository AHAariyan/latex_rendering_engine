//! Textual macro expansion for `\newcommand`, `\renewcommand`,
//! `\providecommand` and `\def`, plus macros supplied by the host.
//!
//! Expansion happens on the source text before lexing. This is simpler than
//! TeX's token-level expansion and covers the way macros are used in practice
//! (shorthand for symbols and small templates with a few arguments).

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub struct MacroDef {
    pub params: usize,
    pub body: String,
}

/// A set of macro definitions supplied by the host application.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Macros {
    defs: Vec<(String, MacroDef)>,
}

impl Macros {
    pub fn new() -> Self {
        Self::default()
    }

    /// Defines `\name`. The parameter count is inferred from the highest `#n` in `body`.
    pub fn define(&mut self, name: &str, body: &str) -> &mut Self {
        let params = max_param(body);
        self.insert(
            name.trim_start_matches('\\').to_string(),
            MacroDef {
                params,
                body: body.to_string(),
            },
        );
        self
    }

    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }

    fn insert(&mut self, name: String, def: MacroDef) {
        if let Some(slot) = self.defs.iter_mut().find(|(n, _)| *n == name) {
            slot.1 = def;
        } else {
            self.defs.push((name, def));
        }
    }

    fn get(&self, name: &str) -> Option<&MacroDef> {
        self.defs.iter().find(|(n, _)| n == name).map(|(_, d)| d)
    }
}

fn max_param(body: &str) -> usize {
    let b = body.as_bytes();
    let mut max = 0;
    let mut i = 0;
    while i + 1 < b.len() {
        if b[i] == b'#' && b[i + 1].is_ascii_digit() {
            max = max.max((b[i + 1] - b'0') as usize);
            i += 2;
        } else {
            i += 1;
        }
    }
    max
}

/// Expansion steps allowed before a macro is treated as runaway.
const MAX_EXPANSIONS: usize = 10_000;

/// Expands macros in `src`. Definitions found in the source are removed from
/// the output. Returns the source unchanged when there is nothing to expand.
pub fn expand(src: &str, host: &Macros) -> Result<String> {
    expand_within(src, host, usize::MAX)
}

/// Expands macros, refusing to produce more than `max_bytes` of source.
pub fn expand_within(src: &str, host: &Macros, max_bytes: usize) -> Result<String> {
    let mut macros = host.clone();
    let mut text = collect_definitions(src, &mut macros)?;
    if macros.is_empty() {
        return Ok(text);
    }
    let mut budget = MAX_EXPANSIONS;
    loop {
        let Some((start, end, name)) = find_macro_use(&text, &macros) else {
            return Ok(text);
        };
        budget -= 1;
        if budget == 0 {
            // Either a recursive macro or one that doubles its output each
            // level; both are a size problem, not a syntax one.
            let _ = (start, &name);
            return Err(Error::TooLarge {
                what: "macro expansions",
                limit: MAX_EXPANSIONS,
            });
        }
        let def = macros.get(&name).unwrap().clone();
        let mut pos = end;
        let mut args = Vec::with_capacity(def.params);
        for _ in 0..def.params {
            let (arg, next) =
                read_argument(&text, pos).ok_or_else(|| Error::parse(start, format!("\\{name} needs {} argument(s)", def.params)))?;
            args.push(arg);
            pos = next;
        }
        let replacement = substitute(&def.body, &args);
        // A macro used before a letter needs a separating space, e.g. `\R x` -> `\mathbb{R} x`.
        let needs_space =
            replacement.ends_with(|c: char| c.is_ascii_alphabetic()) && text[pos..].starts_with(|c: char| c.is_ascii_alphabetic());
        if text.len() + replacement.len() > max_bytes {
            return Err(Error::TooLarge {
                what: "bytes of source after macro expansion",
                limit: max_bytes,
            });
        }
        let mut out = String::with_capacity(text.len() + replacement.len());
        out.push_str(&text[..start]);
        out.push_str(&replacement);
        if needs_space {
            out.push(' ');
        }
        out.push_str(&text[pos..]);
        text = out;
    }
}

fn substitute(body: &str, args: &[String]) -> String {
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' {
            match chars.peek() {
                Some('#') => {
                    chars.next();
                    out.push('#');
                }
                Some(d) if d.is_ascii_digit() => {
                    let idx = (*d as u8 - b'0') as usize;
                    chars.next();
                    if idx >= 1 && idx <= args.len() {
                        out.push_str(&args[idx - 1]);
                    }
                }
                _ => out.push('#'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Finds the earliest `\name` in `text` that is a defined macro and not part
/// of a longer control sequence. Returns (start, end_of_name, name).
fn find_macro_use(text: &str, macros: &Macros) -> Option<(usize, usize, String)> {
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            let start = i;
            i += 1;
            let name_start = i;
            while i < b.len() && b[i].is_ascii_alphabetic() {
                i += 1;
            }
            if i == name_start {
                // Control symbol such as `\\` or `\{`: skip the symbol character.
                i += text[i..].chars().next().map_or(1, |c| c.len_utf8());
                continue;
            }
            let name = &text[name_start..i];
            if macros.get(name).is_some() {
                return Some((start, i, name.to_string()));
            }
        } else {
            i += 1;
        }
    }
    None
}

/// Reads one macro argument at `pos`: a balanced group (returned without
/// braces), a control sequence, or a single character. Leading spaces are skipped.
fn read_argument(text: &str, pos: usize) -> Option<(String, usize)> {
    let b = text.as_bytes();
    let mut i = pos;
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= b.len() {
        return None;
    }
    if b[i] == b'{' {
        let end = balanced_end(text, i)?;
        return Some((text[i + 1..end].to_string(), end + 1));
    }
    if b[i] == b'\\' {
        let mut j = i + 1;
        while j < b.len() && b[j].is_ascii_alphabetic() {
            j += 1;
        }
        if j == i + 1 {
            j += text[j..].chars().next().map_or(1, |c| c.len_utf8());
        }
        return Some((text[i..j].to_string(), j));
    }
    let c = text[i..].chars().next()?;
    Some((c.to_string(), i + c.len_utf8()))
}

/// Index of the `}` matching the `{` at `open`.
fn balanced_end(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut escaped = false;
    for (i, c) in text[open..].char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            '\\' => escaped = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + i);
                }
            }
            _ => {}
        }
    }
    None
}

/// Extracts `\newcommand`, `\renewcommand`, `\providecommand` and `\def`
/// definitions, adds them to `macros`, and returns the source without them.
fn collect_definitions(src: &str, macros: &mut Macros) -> Result<String> {
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    let b = src.as_bytes();
    while i < b.len() {
        let rest = &src[i..];
        let keyword = ["\\newcommand", "\\renewcommand", "\\providecommand", "\\def"]
            .iter()
            .find(|k| rest.starts_with(*k))
            .copied();
        let Some(kw) = keyword else {
            let c = rest.chars().next().unwrap();
            out.push(c);
            i += c.len_utf8();
            continue;
        };
        let after_kw = i + kw.len();
        if kw != "\\def" && src[after_kw..].starts_with(|c: char| c.is_ascii_alphabetic()) {
            // Longer control sequence such as `\newcommandx`; not a definition.
            out.push_str(kw);
            i = after_kw;
            continue;
        }
        let mut j = after_kw;
        if src[j..].starts_with('*') {
            j += 1;
        }
        let (name, params, body, end) = if kw == "\\def" {
            parse_def(src, j).ok_or_else(|| Error::parse(i, "malformed \\def"))?
        } else {
            parse_newcommand(src, j).ok_or_else(|| Error::parse(i, format!("malformed {kw}")))?
        };
        let exists = macros.get(&name).is_some();
        if !(kw == "\\providecommand" && exists) {
            macros.insert(name, MacroDef { params, body });
        }
        i = end;
    }
    Ok(out)
}

fn skip_ws(src: &str, mut i: usize) -> usize {
    let b = src.as_bytes();
    while i < b.len() && b[i].is_ascii_whitespace() {
        i += 1;
    }
    i
}

fn read_cs_name(src: &str, i: usize) -> Option<(String, usize)> {
    let b = src.as_bytes();
    if i >= b.len() || b[i] != b'\\' {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() && b[j].is_ascii_alphabetic() {
        j += 1;
    }
    if j == i + 1 {
        return None;
    }
    Some((src[i + 1..j].to_string(), j))
}

/// `{\name}[n]{body}` or `\name[n]{body}` after the keyword.
fn parse_newcommand(src: &str, i: usize) -> Option<(String, usize, String, usize)> {
    let mut i = skip_ws(src, i);
    let braced = src[i..].starts_with('{');
    if braced {
        i = skip_ws(src, i + 1);
    }
    let (name, mut i) = read_cs_name(src, i)?;
    if braced {
        i = skip_ws(src, i);
        if !src[i..].starts_with('}') {
            return None;
        }
        i += 1;
    }
    i = skip_ws(src, i);
    let mut params = 0;
    if src[i..].starts_with('[') {
        let close = src[i..].find(']')? + i;
        params = src[i + 1..close].trim().parse().ok()?;
        i = skip_ws(src, close + 1);
    }
    // An optional default for the first argument is not supported; skip it.
    if src[i..].starts_with('[') {
        let close = src[i..].find(']')? + i;
        i = skip_ws(src, close + 1);
    }
    if !src[i..].starts_with('{') {
        return None;
    }
    let end = balanced_end(src, i)?;
    Some((name, params, src[i + 1..end].to_string(), end + 1))
}

/// `\name#1#2{body}` after `\def`.
fn parse_def(src: &str, i: usize) -> Option<(String, usize, String, usize)> {
    let i = skip_ws(src, i);
    let (name, mut i) = read_cs_name(src, i)?;
    let mut params = 0;
    loop {
        i = skip_ws(src, i);
        if src[i..].starts_with('#') {
            params += 1;
            i += 2;
        } else {
            break;
        }
    }
    if !src[i..].starts_with('{') {
        return None;
    }
    let end = balanced_end(src, i)?;
    Some((name, params, src[i + 1..end].to_string(), end + 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newcommand_with_arguments() {
        let out = expand(
            r"\newcommand{\R}{\mathbb{R}} \newcommand{\pd}[2]{\frac{\partial #1}{\partial #2}} \pd{f}{x} \in \R",
            &Macros::new(),
        )
        .unwrap();
        assert_eq!(out.trim(), r"\frac{\partial f}{\partial x} \in \mathbb{R}");
    }

    #[test]
    fn def_and_host_macros() {
        let mut host = Macros::new();
        host.define(r"\half", r"\frac{1}{2}");
        let out = expand(r"\def\vec#1{\mathbf{#1}} \vec v + \half", &host).unwrap();
        assert_eq!(out.trim(), r"\mathbf{v} + \frac{1}{2}");
    }

    #[test]
    fn macro_followed_by_letter_gets_a_space() {
        let mut host = Macros::new();
        host.define(r"\eps", r"\varepsilon");
        assert_eq!(expand(r"\eps x", &host).unwrap(), r"\varepsilon x");
        assert_eq!(expand(r"\epsilon", &host).unwrap(), r"\epsilon", "longer names are not touched");
    }

    #[test]
    fn expansion_is_capped() {
        // Doubling macros: each level squares the output.
        let src = r"\def\a{xx}\def\b{\a\a}\def\c{\b\b}\def\d{\c\c}\def\e{\d\d}\e\e\e";
        assert!(expand_within(src, &Macros::new(), 32).is_err());
        assert!(expand_within(src, &Macros::new(), 4096).is_ok());
    }

    #[test]
    fn recursion_is_bounded() {
        assert!(expand(r"\def\a{\a} \a", &Macros::new()).is_err());
    }

    #[test]
    fn untouched_without_macros() {
        assert_eq!(expand(r"x^2", &Macros::new()).unwrap(), "x^2");
    }
}
