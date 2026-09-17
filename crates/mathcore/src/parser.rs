//! Recursive-descent parser: TeX math source -> `Node` tree.
//!
//! The supported command set follows KaTeX's function list. Unknown commands
//! are hard errors rather than silently dropped, because a formula that
//! renders with a missing piece is worse than one that fails loudly.

use crate::ast::*;
use crate::display::Color;
use crate::error::{Error, Result};
use crate::lexer::{Lexer, Tok};
use crate::macros::{self, Macros};
use crate::symbols;

/// Parses a formula with no host-supplied macros.
pub fn parse(src: &str) -> Result<Vec<Node>> {
    parse_with(src, &Macros::new())
}

/// Parses a formula after expanding `macros` and any definitions in the source.
pub fn parse_with(src: &str, macros: &Macros) -> Result<Vec<Node>> {
    let expanded = macros::expand(src, macros)?;
    let mut p = Parser {
        lx: Lexer::new(&expanded),
        variant: Variant::Normal,
        in_left_right: 0,
        depth: 0,
    };
    let nodes = p.parse_list()?;
    match p.lx.advance()? {
        Tok::Eof => Ok(nodes),
        Tok::RBrace => Err(Error::parse(p.lx.pos(), "unexpected `}`")),
        Tok::Amp => Err(Error::parse(p.lx.pos(), "`&` outside of an array")),
        Tok::Cmd(c) => Err(Error::parse(p.lx.pos(), format!("unexpected \\{c}"))),
        t => Err(Error::parse(p.lx.pos(), format!("unexpected token {t:?}"))),
    }
}

struct Parser<'a> {
    lx: Lexer<'a>,
    variant: Variant,
    in_left_right: usize,
    /// Current nesting of groups, arguments and environments.
    depth: usize,
}

/// Deeper nesting than this is rejected. The layout engine recurses once per
/// level, and host threads (Android, JNI, wasm) may have stacks under 1 MB.
const MAX_DEPTH: usize = 64;

/// Tokens that end a list. The stopping token is left unconsumed.
fn is_stop(tok: &Tok<'_>) -> bool {
    matches!(
        tok,
        Tok::Eof | Tok::RBrace | Tok::Amp | Tok::Cmd("\\") | Tok::Cmd("right") | Tok::Cmd("end") | Tok::Cmd("middle") | Tok::Cmd("cr")
    )
}

fn is_infix(tok: &Tok<'_>) -> bool {
    matches!(
        tok,
        Tok::Cmd("over") | Tok::Cmd("choose") | Tok::Cmd("atop") | Tok::Cmd("brace") | Tok::Cmd("brack")
    )
}

/// Parses `2pt`, `1.5em`, `-3mu`, ... into em (1 pt = 0.1 em at TeX's 10 pt base).
pub fn parse_dimen(s: &str) -> Option<f32> {
    let s = s.trim();
    let split = s.find(|c: char| c.is_ascii_alphabetic())?;
    let value: f32 = s[..split].trim().parse().ok()?;
    let unit = s[split..].trim();
    let per_em = match unit {
        "em" => 1.0,
        "ex" => 0.43,
        "pt" => 0.1,
        "bp" => 0.1004,
        "mu" => 1.0 / 18.0,
        "px" => 0.075,
        "mm" => 0.2845,
        "cm" => 2.845,
        "in" => 7.227,
        "pc" => 1.2,
        "dd" => 0.107,
        _ => return None,
    };
    Some(value * per_em)
}

impl<'a> Parser<'a> {
    /// Parses atoms until a group/row/environment terminator.
    fn parse_list(&mut self) -> Result<Vec<Node>> {
        if self.depth > MAX_DEPTH {
            return Err(Error::parse(self.lx.pos(), format!("nesting deeper than {MAX_DEPTH} levels")));
        }
        self.depth += 1;
        let r = self.parse_list_inner();
        self.depth -= 1;
        r
    }

    fn parse_list_inner(&mut self) -> Result<Vec<Node>> {
        let mut out = Vec::new();
        loop {
            let tok = self.lx.peek()?;
            if is_stop(tok) {
                return Ok(out);
            }
            if is_infix(tok) {
                let pos = self.lx.pos();
                let Tok::Cmd(name) = self.lx.advance()? else { unreachable!() };
                let den = self.parse_list()?;
                let delims = match name {
                    "choose" => Some((Some('('), Some(')'))),
                    "brace" => Some((Some('{'), Some('}'))),
                    "brack" => Some((Some('['), Some(']'))),
                    _ => None,
                };
                let node = Node::Frac {
                    num: Box::new(Node::Row(out)),
                    den: Box::new(Node::Row(den)),
                    rule: if name == "over" { FracRule::Default } else { FracRule::None },
                    style: None,
                    delims,
                };
                let _ = pos;
                return Ok(vec![node]);
            }
            let node = self.parse_atom()?;
            out.push(node);
        }
    }

    /// One nucleus plus any scripts and primes attached to it.
    fn parse_atom(&mut self) -> Result<Node> {
        let pos = self.lx.pos();
        let base = self.parse_nucleus()?;
        let mut sup: Option<Node> = None;
        let mut sub: Option<Node> = None;
        loop {
            match self.lx.peek()? {
                Tok::Sup => {
                    self.lx.advance()?;
                    let arg = self.parse_arg()?;
                    match sup.take() {
                        None => sup = Some(arg),
                        Some(Node::Symbol { ch, .. }) if is_prime(ch) => {
                            sup = Some(Node::Row(vec![
                                Node::Symbol {
                                    ch,
                                    atom: AtomType::Ord,
                                    variant: Variant::Normal,
                                },
                                arg,
                            ]));
                        }
                        Some(_) => return Err(Error::parse(pos, "double superscript")),
                    }
                }
                Tok::Sub => {
                    self.lx.advance()?;
                    if sub.is_some() {
                        return Err(Error::parse(pos, "double subscript"));
                    }
                    sub = Some(self.parse_arg()?);
                }
                Tok::Char('\'') => {
                    let mut n = 0;
                    while matches!(self.lx.peek()?, Tok::Char('\'')) {
                        self.lx.advance()?;
                        n += 1;
                    }
                    let ch = match n {
                        1 => '′',
                        2 => '″',
                        3 => '‴',
                        _ => '⁗',
                    };
                    let prime = Node::Symbol {
                        ch,
                        atom: AtomType::Ord,
                        variant: Variant::Normal,
                    };
                    sup = Some(match sup.take() {
                        None => prime,
                        Some(existing) => Node::Row(vec![existing, prime]),
                    });
                }
                _ => break,
            }
        }
        if sup.is_none() && sub.is_none() {
            return Ok(base);
        }
        Ok(Node::Scripts {
            base: Box::new(base),
            sup: sup.map(Box::new),
            sub: sub.map(Box::new),
        })
    }

    /// A single argument: a braced group or one token.
    fn parse_arg(&mut self) -> Result<Node> {
        let pos = self.lx.pos();
        if self.depth > MAX_DEPTH {
            return Err(Error::parse(pos, format!("nesting deeper than {MAX_DEPTH} levels")));
        }
        self.depth += 1;
        let r = self.parse_arg_inner(pos);
        self.depth -= 1;
        r
    }

    fn parse_arg_inner(&mut self, pos: usize) -> Result<Node> {
        match self.lx.peek()? {
            Tok::LBrace => {
                self.lx.advance()?;
                let saved = self.variant;
                let list = self.parse_list()?;
                self.variant = saved;
                self.expect_rbrace()?;
                Ok(Node::Row(list))
            }
            Tok::Char(_) | Tok::Cmd(_) => self.parse_nucleus(),
            t => Err(Error::parse(pos, format!("expected an argument, found {t:?}"))),
        }
    }

    fn expect_rbrace(&mut self) -> Result<()> {
        let pos = self.lx.pos();
        match self.lx.advance()? {
            Tok::RBrace => Ok(()),
            Tok::Eof => Err(Error::parse(pos, "missing `}`")),
            t => Err(Error::parse(pos, format!("expected `}}`, found {t:?}"))),
        }
    }

    fn parse_nucleus(&mut self) -> Result<Node> {
        let pos = self.lx.pos();
        match self.lx.advance()? {
            Tok::Char(c) => Ok(self.char_node(c)),
            Tok::LBrace => {
                let saved = self.variant;
                let list = self.parse_list()?;
                self.variant = saved;
                self.expect_rbrace()?;
                Ok(Node::Row(list))
            }
            Tok::Cmd(name) => self.parse_command(name, pos),
            Tok::Sup | Tok::Sub => Err(Error::parse(pos, "missing base for script")),
            Tok::RBrace => Err(Error::parse(pos, "unexpected `}`")),
            Tok::Amp => Err(Error::parse(pos, "`&` outside of an array")),
            Tok::Eof => Err(Error::parse(pos, "unexpected end of input")),
        }
    }

    fn char_node(&self, c: char) -> Node {
        match c {
            '~' => Node::Space { mu: 6.0 },
            '\'' => Node::Symbol {
                ch: '′',
                atom: AtomType::Ord,
                variant: Variant::Normal,
            },
            _ => {
                if let Some(limits) = symbols::char_big_op(c) {
                    return Node::BigOp {
                        ch: c,
                        limits: if limits { Limits::Default } else { Limits::NoLimits },
                    };
                }
                let ch = match c {
                    '-' => '−',
                    '*' => '∗',
                    _ => c,
                };
                Node::Symbol {
                    ch,
                    atom: symbols::char_atom(c),
                    variant: self.variant,
                }
            }
        }
    }

    /// Reads a dimension given either as a group `{2pt}` or as bare tokens `2pt`.
    fn parse_dimen_arg(&mut self, pos: usize) -> Result<f32> {
        if matches!(self.lx.peek()?, Tok::LBrace) {
            let raw = self.lx.raw_group()?;
            return parse_dimen(raw).ok_or_else(|| Error::parse(pos, format!("bad dimension `{raw}`")));
        }
        let mut s = String::new();
        let mut letters = 0;
        loop {
            match self.lx.peek()? {
                Tok::Char(c) if c.is_ascii_digit() || matches!(c, '.' | '-' | '+') => {
                    if letters > 0 {
                        break;
                    }
                    s.push(*c);
                    self.lx.advance()?;
                }
                Tok::Char(c) if c.is_ascii_alphabetic() && letters < 2 => {
                    s.push(*c);
                    letters += 1;
                    self.lx.advance()?;
                    if letters == 2 {
                        break;
                    }
                }
                _ => break,
            }
        }
        parse_dimen(&s).ok_or_else(|| Error::parse(pos, format!("bad dimension `{s}`")))
    }

    fn parse_color_arg(&mut self, pos: usize) -> Result<Color> {
        let raw = self.lx.raw_group()?;
        let rgb = symbols::parse_color(raw).ok_or_else(|| Error::parse(pos, format!("unknown color `{raw}`")))?;
        Ok(Color(rgb[0], rgb[1], rgb[2], 255))
    }

    fn parse_command(&mut self, name: &'a str, pos: usize) -> Result<Node> {
        if let Some((_, mu)) = symbols::SPACES.iter().find(|(n, _)| *n == name) {
            return Ok(Node::Space { mu: *mu });
        }
        if let Some((ch, atom)) = symbols::lookup_symbol(name) {
            return Ok(Node::Symbol {
                ch,
                atom,
                variant: self.variant,
            });
        }
        if let Some((_, ch, limits)) = symbols::BIG_OPS.iter().find(|(n, _, _)| *n == name) {
            let limits = self.parse_limits_modifier(if *limits { Limits::Default } else { Limits::NoLimits })?;
            return Ok(Node::BigOp { ch: *ch, limits });
        }
        if let Some((_, limits)) = symbols::FN_NAMES.iter().find(|(n, _)| *n == name) {
            let limits = self.parse_limits_modifier(if *limits { Limits::Default } else { Limits::NoLimits })?;
            return Ok(Node::FnName {
                name: name.to_string(),
                limits,
            });
        }
        if let Some((_, ch, stretchy)) = symbols::ACCENTS.iter().find(|(n, _, _)| *n == name) {
            let base = self.parse_arg()?;
            return Ok(Node::Accent {
                ch: *ch,
                base: Box::new(base),
                stretchy: *stretchy,
            });
        }
        // `\varGamma` etc.: italic uppercase Greek.
        if let Some(greek) = name.strip_prefix("var") {
            if let Some((ch, _)) = symbols::lookup_symbol(greek).filter(|(c, _)| ('Α'..='Ω').contains(c)) {
                return Ok(Node::Symbol {
                    ch,
                    atom: AtomType::Ord,
                    variant: Variant::Italic,
                });
            }
        }
        match name {
            "frac" | "dfrac" | "tfrac" | "cfrac" => {
                let num = self.parse_arg()?;
                let den = self.parse_arg()?;
                let style = match name {
                    "dfrac" | "cfrac" => Some(MathStyle::Display),
                    "tfrac" => Some(MathStyle::Text),
                    _ => None,
                };
                Ok(Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    rule: FracRule::Default,
                    style,
                    delims: None,
                })
            }
            "binom" | "dbinom" | "tbinom" => {
                let num = self.parse_arg()?;
                let den = self.parse_arg()?;
                let style = match name {
                    "dbinom" => Some(MathStyle::Display),
                    "tbinom" => Some(MathStyle::Text),
                    _ => None,
                };
                Ok(Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    rule: FracRule::None,
                    style,
                    delims: Some((Some('('), Some(')'))),
                })
            }
            "genfrac" => {
                let left = self.lx.raw_group()?.trim().to_string();
                let right = self.lx.raw_group()?.trim().to_string();
                let thickness = self.lx.raw_group()?.trim().to_string();
                let style = self.lx.raw_group()?.trim().to_string();
                let num = self.parse_arg()?;
                let den = self.parse_arg()?;
                let delim = |s: &str| -> Result<Delim> {
                    if s.is_empty() || s == "." {
                        Ok(None)
                    } else {
                        symbols::delimiter(s.trim_start_matches('\\'))
                            .map(Some)
                            .ok_or_else(|| Error::parse(pos, format!("`{s}` is not a delimiter")))
                    }
                };
                let rule = if thickness.is_empty() {
                    FracRule::Default
                } else {
                    let em = parse_dimen(&thickness).ok_or_else(|| Error::parse(pos, format!("bad thickness `{thickness}`")))?;
                    if em <= 0.0 {
                        FracRule::None
                    } else {
                        FracRule::Custom(em)
                    }
                };
                let style = match style.as_str() {
                    "0" => Some(MathStyle::Display),
                    "1" => Some(MathStyle::Text),
                    "2" => Some(MathStyle::Script),
                    "3" => Some(MathStyle::ScriptScript),
                    _ => None,
                };
                let (l, r) = (delim(&left)?, delim(&right)?);
                let delims = if l.is_some() || r.is_some() { Some((l, r)) } else { None };
                Ok(Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    rule,
                    style,
                    delims,
                })
            }
            "sqrt" => {
                let index = if matches!(self.lx.peek()?, Tok::Char('[')) {
                    self.lx.advance()?;
                    let mut items = Vec::new();
                    loop {
                        match self.lx.peek()? {
                            Tok::Char(']') => {
                                self.lx.advance()?;
                                break;
                            }
                            Tok::Eof => return Err(Error::parse(pos, "missing `]` in \\sqrt")),
                            _ => items.push(self.parse_atom()?),
                        }
                    }
                    Some(Box::new(Node::Row(items)))
                } else {
                    None
                };
                let radicand = self.parse_arg()?;
                Ok(Node::Sqrt {
                    radicand: Box::new(radicand),
                    index,
                })
            }
            "left" => {
                let left = self.parse_delimiter(pos)?;
                self.in_left_right += 1;
                let mut body = self.parse_list()?;
                loop {
                    match self.lx.advance()? {
                        Tok::Cmd("right") => break,
                        Tok::Cmd("middle") => {
                            let ch = self
                                .parse_delimiter(pos)?
                                .ok_or_else(|| Error::parse(pos, "\\middle needs a delimiter"))?;
                            body.push(Node::Middle(ch));
                            body.extend(self.parse_list()?);
                        }
                        _ => return Err(Error::parse(pos, "\\left without matching \\right")),
                    }
                }
                self.in_left_right -= 1;
                let right = self.parse_delimiter(pos)?;
                Ok(Node::LeftRight { left, body, right })
            }
            "right" => Err(Error::parse(pos, "\\right without matching \\left")),
            "middle" => Err(Error::parse(pos, "\\middle outside of \\left ... \\right")),
            "big" | "Big" | "bigg" | "Bigg" | "bigl" | "Bigl" | "biggl" | "Biggl" | "bigr" | "Bigr" | "biggr" | "Biggr" | "bigm"
            | "Bigm" | "biggm" | "Biggm" => {
                let base = name.trim_end_matches(['l', 'r', 'm']);
                let size = match base {
                    "big" => 1,
                    "Big" => 2,
                    "bigg" => 3,
                    _ => 4,
                };
                let atom = match name.chars().last() {
                    Some('l') => AtomType::Open,
                    Some('r') => AtomType::Close,
                    Some('m') => AtomType::Rel,
                    _ => AtomType::Ord,
                };
                let ch = self
                    .parse_delimiter(pos)?
                    .ok_or_else(|| Error::parse(pos, "null delimiter after \\big"))?;
                Ok(Node::SizedDelim { ch, size, atom })
            }
            "mathbf" | "mathrm" | "mathit" | "mathbb" | "mathcal" | "mathfrak" | "mathsf" | "mathtt" | "boldsymbol" | "bm"
            | "mathnormal" | "mathscr" | "pmb" | "Bbb" | "bold" | "rm" | "bf" | "it" | "cal" => {
                let v = match name {
                    "mathbf" | "pmb" | "bold" | "bf" => Variant::Bold,
                    "mathrm" | "rm" => Variant::Roman,
                    "mathit" | "it" => Variant::Italic,
                    "mathbb" | "Bbb" => Variant::DoubleStruck,
                    "mathcal" | "mathscr" | "cal" => Variant::Script,
                    "mathfrak" => Variant::Fraktur,
                    "mathsf" => Variant::SansSerif,
                    "mathtt" => Variant::Monospace,
                    "boldsymbol" | "bm" => Variant::BoldItalic,
                    _ => Variant::Normal,
                };
                if matches!(name, "rm" | "bf" | "it" | "cal") {
                    // Plain TeX font switches apply to the rest of the group.
                    self.variant = v;
                    let body = self.parse_list()?;
                    return Ok(Node::Row(body));
                }
                let saved = self.variant;
                self.variant = v;
                let arg = self.parse_arg();
                self.variant = saved;
                arg
            }
            "text" | "textrm" | "textnormal" | "mbox" | "textit" | "textbf" | "textsf" | "texttt" | "hbox" => {
                let raw = self.lx.raw_group()?;
                let variant = match name {
                    "textit" => Variant::Italic,
                    "textbf" => Variant::Bold,
                    "textsf" => Variant::SansSerif,
                    "texttt" => Variant::Monospace,
                    _ => Variant::Roman,
                };
                Ok(Node::Text {
                    text: raw.to_string(),
                    variant,
                })
            }
            "operatorname" | "operatornamewithlimits" => {
                let starred = if matches!(self.lx.peek()?, Tok::Char('*')) {
                    self.lx.advance()?;
                    true
                } else {
                    false
                };
                let raw = self.lx.raw_group()?.trim().to_string();
                let default = if name == "operatorname" && !starred {
                    Limits::NoLimits
                } else {
                    Limits::Default
                };
                let limits = self.parse_limits_modifier(default)?;
                Ok(Node::FnName { name: raw, limits })
            }
            "mathop" | "mathrel" | "mathbin" | "mathord" | "mathopen" | "mathclose" | "mathpunct" | "mathinner" => {
                let atom = match name {
                    "mathop" => AtomType::Op,
                    "mathrel" => AtomType::Rel,
                    "mathbin" => AtomType::Bin,
                    "mathopen" => AtomType::Open,
                    "mathclose" => AtomType::Close,
                    "mathpunct" => AtomType::Punct,
                    "mathinner" => AtomType::Inner,
                    _ => AtomType::Ord,
                };
                let body = self.parse_arg()?;
                let limits = if atom == AtomType::Op {
                    self.parse_limits_modifier(Limits::Default)?
                } else {
                    Limits::NoLimits
                };
                Ok(Node::Class {
                    atom,
                    body: Box::new(body),
                    limits,
                })
            }
            "displaystyle" | "textstyle" | "scriptstyle" | "scriptscriptstyle" => {
                let style = match name {
                    "displaystyle" => MathStyle::Display,
                    "textstyle" => MathStyle::Text,
                    "scriptstyle" => MathStyle::Script,
                    _ => MathStyle::ScriptScript,
                };
                let body = self.parse_list()?;
                Ok(Node::Style { style, body })
            }
            "overline" => Ok(Node::Overline(Box::new(self.parse_arg()?))),
            "underline" => Ok(Node::Underline(Box::new(self.parse_arg()?))),
            "phantom" | "hphantom" | "vphantom" | "smash" => {
                let kind = match name {
                    "hphantom" => PhantomKind::Horizontal,
                    "vphantom" => PhantomKind::Vertical,
                    "smash" => PhantomKind::Smash,
                    _ => PhantomKind::Full,
                };
                if name == "smash" && matches!(self.lx.peek()?, Tok::Char('[')) {
                    while !matches!(self.lx.advance()?, Tok::Char(']') | Tok::Eof) {}
                }
                Ok(Node::Phantom {
                    body: Box::new(self.parse_arg()?),
                    kind,
                })
            }
            "mathstrut" | "strut" => Ok(Node::Phantom {
                body: Box::new(Node::Symbol {
                    ch: '(',
                    atom: AtomType::Ord,
                    variant: Variant::Normal,
                }),
                kind: PhantomKind::Vertical,
            }),
            "limits" | "nolimits" | "displaylimits" | "relax" | "nonumber" | "notag" | "allowbreak" | "noindent" | "ignorespaces" => {
                Ok(Node::Row(vec![]))
            }
            "label" | "tag" | "ref" | "eqref" => {
                let _ = self.lx.raw_group()?;
                Ok(Node::Row(vec![]))
            }
            "not" => {
                let arg = self.parse_arg()?;
                match arg {
                    Node::Symbol { ch, atom, variant } => {
                        let negated = match ch {
                            '=' => '≠',
                            '∈' => '∉',
                            '<' => '≮',
                            '>' => '≯',
                            '≡' => '≢',
                            '⊂' => '⊄',
                            '⊃' => '⊅',
                            '⊆' => '⊈',
                            '⊇' => '⊉',
                            '∼' => '≁',
                            '≈' => '≉',
                            '≤' => '≰',
                            '≥' => '≱',
                            '∃' => '∄',
                            '≃' => '≄',
                            '≅' => '≇',
                            '∣' => '∤',
                            '∥' => '∦',
                            '→' => '↛',
                            '⇒' => '⇏',
                            '∋' => '∌',
                            other => return Err(Error::parse(pos, format!("\\not cannot negate `{other}`"))),
                        };
                        Ok(Node::Symbol {
                            ch: negated,
                            atom,
                            variant,
                        })
                    }
                    _ => Err(Error::parse(pos, "\\not needs a symbol")),
                }
            }
            "begin" => {
                let env = self.lx.raw_group()?.to_string();
                self.parse_environment(&env, pos)
            }
            "end" => Err(Error::parse(pos, "\\end without matching \\begin")),
            "\\" | "cr" => Err(Error::parse(pos, "\\\\ outside of an array")),
            "overset" | "underset" | "stackrel" => {
                let top = self.parse_arg()?;
                let base = self.parse_arg()?;
                let (over, under) = match name {
                    "underset" => (None, Some(Box::new(top))),
                    _ => (Some(Box::new(top)), None),
                };
                Ok(Node::OverUnder {
                    base: Box::new(base),
                    over,
                    under,
                })
            }
            "substack" => self.in_env_rows("substack", pos, RowPitch::Substack),
            "color" => {
                let color = self.parse_color_arg(pos)?;
                let body = self.parse_list()?;
                Ok(Node::Color { color, body })
            }
            "textcolor" => {
                let color = self.parse_color_arg(pos)?;
                let body = self.parse_arg()?;
                Ok(Node::Color { color, body: vec![body] })
            }
            "boxed" | "fbox" => Ok(Node::Boxed(Box::new(self.parse_arg()?))),
            "cancel" | "bcancel" | "xcancel" => {
                let kind = match name {
                    "bcancel" => CancelKind::Down,
                    "xcancel" => CancelKind::Cross,
                    _ => CancelKind::Up,
                };
                Ok(Node::Cancel {
                    body: Box::new(self.parse_arg()?),
                    kind,
                })
            }
            "underbrace" | "overbrace" => {
                let base = self.parse_arg()?;
                Ok(Node::HBrace {
                    base: Box::new(base),
                    over: name == "overbrace",
                })
            }
            "xrightarrow" | "xleftarrow" | "xleftrightarrow" | "xRightarrow" | "xLeftarrow" | "xLeftrightarrow" | "xmapsto"
            | "xhookrightarrow" | "xhookleftarrow" | "xtwoheadrightarrow" | "xtwoheadleftarrow" | "xrightharpoonup" | "xleftharpoonup"
            | "xlongequal" => {
                let ch = match name {
                    "xrightarrow" => '→',
                    "xleftarrow" => '←',
                    "xleftrightarrow" => '↔',
                    "xRightarrow" => '⇒',
                    "xLeftarrow" => '⇐',
                    "xLeftrightarrow" => '⇔',
                    "xmapsto" => '↦',
                    "xhookrightarrow" => '↪',
                    "xhookleftarrow" => '↩',
                    "xtwoheadrightarrow" => '↠',
                    "xtwoheadleftarrow" => '↞',
                    "xrightharpoonup" => '⇀',
                    "xleftharpoonup" => '↼',
                    _ => '=',
                };
                let under = if matches!(self.lx.peek()?, Tok::Char('[')) {
                    self.lx.advance()?;
                    let mut items = Vec::new();
                    loop {
                        match self.lx.peek()? {
                            Tok::Char(']') => {
                                self.lx.advance()?;
                                break;
                            }
                            Tok::Eof => return Err(Error::parse(pos, "missing `]`")),
                            _ => items.push(self.parse_atom()?),
                        }
                    }
                    Some(Box::new(Node::Row(items)))
                } else {
                    None
                };
                let over = self.parse_arg()?;
                Ok(Node::XArrow {
                    ch,
                    over: Some(Box::new(over)),
                    under,
                })
            }
            "pmod" | "pod" | "mod" | "bmod" => {
                let mut items = Vec::new();
                match name {
                    "bmod" => {
                        return Ok(Node::Class {
                            atom: AtomType::Bin,
                            body: Box::new(Node::Text {
                                text: "mod".into(),
                                variant: Variant::Roman,
                            }),
                            limits: Limits::NoLimits,
                        });
                    }
                    "pmod" | "pod" => {
                        let arg = self.parse_arg()?;
                        items.push(Node::Space { mu: 18.0 });
                        items.push(Node::Symbol {
                            ch: '(',
                            atom: AtomType::Open,
                            variant: Variant::Normal,
                        });
                        if name == "pmod" {
                            items.push(Node::Text {
                                text: "mod".into(),
                                variant: Variant::Roman,
                            });
                            items.push(Node::Space { mu: 6.0 });
                        }
                        items.push(arg);
                        items.push(Node::Symbol {
                            ch: ')',
                            atom: AtomType::Close,
                            variant: Variant::Normal,
                        });
                    }
                    _ => {
                        let arg = self.parse_arg()?;
                        items.push(Node::Space { mu: 18.0 });
                        items.push(Node::Text {
                            text: "mod".into(),
                            variant: Variant::Roman,
                        });
                        items.push(Node::Space { mu: 6.0 });
                        items.push(arg);
                    }
                }
                Ok(Node::Row(items))
            }
            "hspace" | "hskip" | "kern" | "mkern" | "mskip" | "hspace*" => {
                let em = self.parse_dimen_arg(pos)?;
                Ok(Node::Space { mu: em * 18.0 })
            }
            "lVert" | "rVert" => Ok(Node::Symbol {
                ch: '‖',
                atom: if name == "lVert" { AtomType::Open } else { AtomType::Close },
                variant: Variant::Normal,
            }),
            _ => Err(Error::parse(pos, format!("unknown command \\{name}"))),
        }
    }

    fn parse_limits_modifier(&mut self, default: Limits) -> Result<Limits> {
        Ok(match self.lx.peek()? {
            Tok::Cmd("limits") => {
                self.lx.advance()?;
                Limits::Limits
            }
            Tok::Cmd("nolimits") => {
                self.lx.advance()?;
                Limits::NoLimits
            }
            _ => default,
        })
    }

    fn parse_delimiter(&mut self, pos: usize) -> Result<Delim> {
        match self.lx.advance()? {
            Tok::Char('.') => Ok(None),
            Tok::Char(c) => symbols::delimiter(&c.to_string())
                .map(Some)
                .ok_or_else(|| Error::parse(pos, format!("`{c}` is not a delimiter"))),
            Tok::Cmd(name) => symbols::delimiter(name)
                .map(Some)
                .ok_or_else(|| Error::parse(pos, format!("\\{name} is not a delimiter"))),
            t => Err(Error::parse(pos, format!("expected a delimiter, found {t:?}"))),
        }
    }

    /// Parses `\substack{...}` bodies: rows separated by `\\`, no `&`.
    fn in_env_rows(&mut self, what: &str, pos: usize, pitch: RowPitch) -> Result<Node> {
        match self.lx.advance()? {
            Tok::LBrace => {}
            _ => return Err(Error::parse(pos, format!("\\{what} needs a group"))),
        }
        let mut rows = vec![];
        loop {
            let cell = self.parse_list()?;
            rows.push(vec![cell]);
            match self.lx.advance()? {
                Tok::Cmd("\\") | Tok::Cmd("cr") => continue,
                Tok::RBrace => break,
                Tok::Eof => return Err(Error::parse(pos, format!("unterminated \\{what}"))),
                t => return Err(Error::parse(pos, format!("unexpected {t:?} in \\{what}"))),
            }
        }
        if rows.len() > 1 && rows.last().is_some_and(|r| r[0].is_empty()) {
            rows.pop();
        }
        Ok(Node::Array(Box::new(Array {
            rows,
            cols: vec![ColAlign::Center],
            cell_style: MathStyle::Script,
            hlines: vec![],
            vlines: vec![],
            row_gaps: vec![],
            pitch,
            stretch: 1.0,
        })))
    }

    fn parse_environment(&mut self, env: &str, pos: usize) -> Result<Node> {
        let mut vlines = Vec::new();
        let (cols, cell_style, left, right): (Vec<ColAlign>, MathStyle, Delim, Delim) = match env {
            "matrix" | "pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix" | "smallmatrix" | "matrix*" | "pmatrix*" | "bmatrix*"
            | "Bmatrix*" | "vmatrix*" | "Vmatrix*" => {
                let (l, r) = match env.trim_end_matches('*') {
                    "pmatrix" => (Some('('), Some(')')),
                    "bmatrix" => (Some('['), Some(']')),
                    "Bmatrix" => (Some('{'), Some('}')),
                    "vmatrix" => (Some('|'), Some('|')),
                    "Vmatrix" => (Some('‖'), Some('‖')),
                    _ => (None, None),
                };
                let mut cols = vec![];
                if env.ends_with('*') && matches!(self.lx.peek()?, Tok::Char('[')) {
                    self.lx.advance()?;
                    let mut align = ColAlign::Center;
                    loop {
                        match self.lx.advance()? {
                            Tok::Char(']') => break,
                            Tok::Char('l') => align = ColAlign::Left,
                            Tok::Char('r') => align = ColAlign::Right,
                            Tok::Char('c') => align = ColAlign::Center,
                            Tok::Eof => return Err(Error::parse(pos, "missing `]`")),
                            _ => {}
                        }
                    }
                    cols = vec![align; 64];
                }
                let style = if env == "smallmatrix" { MathStyle::Script } else { MathStyle::Text };
                (cols, style, l, r)
            }
            "cases" | "dcases" => (
                vec![ColAlign::Left, ColAlign::Left],
                if env == "dcases" { MathStyle::Display } else { MathStyle::Text },
                Some('{'),
                None,
            ),
            "rcases" | "drcases" => (
                vec![ColAlign::Left, ColAlign::Left],
                if env == "drcases" { MathStyle::Display } else { MathStyle::Text },
                None,
                Some('}'),
            ),
            "array" | "darray" | "tabular" => {
                let spec = self.lx.raw_group()?;
                let mut cols = Vec::new();
                for c in spec.chars() {
                    match c {
                        'l' => cols.push(ColAlign::Left),
                        'c' => cols.push(ColAlign::Center),
                        'r' => cols.push(ColAlign::Right),
                        '|' => vlines.push(cols.len()),
                        _ => {}
                    }
                }
                (cols, if env == "darray" { MathStyle::Display } else { MathStyle::Text }, None, None)
            }
            "aligned" | "align" | "align*" | "split" | "alignat" | "alignat*" | "alignedat" | "flalign" | "flalign*" | "eqnarray"
            | "eqnarray*" => {
                if env.starts_with("alignat") || env.starts_with("alignedat") {
                    let _ = self.lx.raw_group()?; // number of alignment pairs, not needed
                }
                (vec![], MathStyle::Display, None, None)
            }
            "gathered" | "gather" | "gather*" | "multline" | "multline*" => (vec![ColAlign::Center], MathStyle::Display, None, None),
            "equation" | "equation*" | "displaymath" | "math" => {
                let body = self.parse_list()?;
                self.expect_end(env, pos)?;
                return Ok(Node::Style {
                    style: MathStyle::Display,
                    body,
                });
            }
            _ => return Err(Error::parse(pos, format!("unknown environment `{env}`"))),
        };

        let is_aligned = matches!(cell_style, MathStyle::Display) && cols.is_empty();
        let mut rows: Vec<Vec<Vec<Node>>> = vec![vec![]];
        let mut hlines = Vec::new();
        let mut row_gaps: Vec<f32> = Vec::new();
        loop {
            // `\hline` is only meaningful at the start of a row.
            if rows.last().unwrap().is_empty() {
                while matches!(self.lx.peek()?, Tok::Cmd("hline") | Tok::Cmd("hdashline")) {
                    self.lx.advance()?;
                    let idx = rows.len() - 1;
                    if !hlines.contains(&idx) {
                        hlines.push(idx);
                    }
                }
            }
            let mut cell = self.parse_list()?;
            // amsmath starts every even column with `{}` so that a leading
            // relation (`a &= b`) still receives its left-hand spacing.
            if is_aligned && rows.last().unwrap().len() % 2 == 1 {
                cell.insert(0, Node::Row(vec![]));
            }
            rows.last_mut().unwrap().push(cell);
            match self.lx.advance()? {
                Tok::Amp => {}
                Tok::Cmd("\\") | Tok::Cmd("cr") => {
                    let mut gap = 0.0;
                    if matches!(self.lx.peek()?, Tok::Char('[')) {
                        self.lx.advance()?;
                        let mut s = String::new();
                        loop {
                            match self.lx.advance()? {
                                Tok::Char(']') => break,
                                Tok::Char(c) => s.push(c),
                                Tok::Eof => return Err(Error::parse(pos, "missing `]`")),
                                _ => {}
                            }
                        }
                        gap = parse_dimen(&s).ok_or_else(|| Error::parse(pos, format!("bad row spacing `{s}`")))?;
                    }
                    while row_gaps.len() < rows.len() - 1 {
                        row_gaps.push(0.0);
                    }
                    row_gaps.push(gap);
                    rows.push(vec![]);
                }
                Tok::Cmd("end") => {
                    let closing = self.lx.raw_group()?;
                    if closing != env {
                        return Err(Error::parse(pos, format!("\\begin{{{env}}} closed by \\end{{{closing}}}")));
                    }
                    break;
                }
                Tok::Eof => return Err(Error::parse(pos, format!("missing \\end{{{env}}}"))),
                Tok::RBrace => return Err(Error::parse(pos, "unexpected `}` inside environment")),
                t => return Err(Error::parse(pos, format!("unexpected {t:?} inside environment"))),
            }
        }
        // A trailing `\\` leaves an empty last row; drop it (keeping a bottom `\hline`).
        if rows.len() > 1 && rows.last().is_some_and(|r| r.iter().all(|c| c.is_empty())) {
            rows.pop();
        }
        let ncols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
        let cols = if cols.is_empty() {
            (0..ncols)
                .map(|i| {
                    if is_aligned {
                        if i % 2 == 0 {
                            ColAlign::Right
                        } else {
                            ColAlign::Left
                        }
                    } else {
                        ColAlign::Center
                    }
                })
                .collect()
        } else {
            cols.into_iter().take(ncols.max(1)).collect()
        };
        let pitch = if env == "smallmatrix" {
            RowPitch::SmallMatrix
        } else {
            RowPitch::Normal
        };
        let stretch = if env.ends_with("cases") { 1.2 } else { 1.0 };
        let array = Node::Array(Box::new(Array {
            rows,
            cols,
            cell_style,
            hlines,
            vlines,
            row_gaps,
            pitch,
            stretch,
        }));
        if left.is_some() || right.is_some() {
            Ok(Node::LeftRight {
                left,
                body: vec![array],
                right,
            })
        } else {
            Ok(array)
        }
    }

    fn expect_end(&mut self, env: &str, pos: usize) -> Result<()> {
        match self.lx.advance()? {
            Tok::Cmd("end") => {
                let closing = self.lx.raw_group()?;
                if closing == env {
                    Ok(())
                } else {
                    Err(Error::parse(pos, format!("\\begin{{{env}}} closed by \\end{{{closing}}}")))
                }
            }
            _ => Err(Error::parse(pos, format!("missing \\end{{{env}}}"))),
        }
    }
}

fn is_prime(ch: char) -> bool {
    matches!(ch, '′' | '″' | '‴' | '⁗')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sym(ch: char, atom: AtomType) -> Node {
        Node::Symbol {
            ch,
            atom,
            variant: Variant::Normal,
        }
    }

    #[test]
    fn scripts_and_primes() {
        let n = parse("x^2_i").unwrap();
        assert_eq!(
            n,
            vec![Node::Scripts {
                base: Box::new(sym('x', AtomType::Ord)),
                sup: Some(Box::new(sym('2', AtomType::Ord))),
                sub: Some(Box::new(sym('i', AtomType::Ord))),
            }]
        );
        let n = parse("f''").unwrap();
        assert!(matches!(&n[0], Node::Scripts { sup: Some(s), .. } if **s == sym('″', AtomType::Ord)));
        assert!(parse("x^1^2").is_err());
    }

    #[test]
    fn fraction_and_sqrt() {
        let n = parse(r"\frac{a}{b} \sqrt[3]{x}").unwrap();
        assert_eq!(n.len(), 2);
        assert!(matches!(
            &n[0],
            Node::Frac {
                rule: FracRule::Default,
                ..
            }
        ));
        assert!(matches!(&n[1], Node::Sqrt { index: Some(_), .. }));
    }

    #[test]
    fn infix_over_and_choose() {
        let n = parse(r"{a \over b}").unwrap();
        assert!(matches!(&n[0], Node::Row(v) if matches!(v[0], Node::Frac { rule: FracRule::Default, .. })));
        let n = parse(r"{n \choose k}").unwrap();
        assert!(matches!(&n[0], Node::Row(v) if matches!(v[0], Node::Frac { delims: Some(_), .. })));
    }

    #[test]
    fn left_right_and_matrix() {
        let n = parse(r"\left( \begin{matrix} a & b \\ c & d \end{matrix} \right)").unwrap();
        let Node::LeftRight { left, body, right } = &n[0] else {
            panic!("{n:?}")
        };
        assert_eq!((*left, *right), (Some('('), Some(')')));
        let Node::Array(a) = &body[0] else { panic!() };
        assert_eq!(a.rows.len(), 2);
        assert_eq!(a.cols.len(), 2);
        assert!(parse(r"\left( x").is_err());
        assert!(parse(r"\begin{matrix} a \end{pmatrix}").is_err());
    }

    #[test]
    fn middle_and_array_rules() {
        let n = parse(r"\left\{ x \middle| y \right\}").unwrap();
        let Node::LeftRight { body, .. } = &n[0] else { panic!() };
        assert!(body.iter().any(|n| matches!(n, Node::Middle('|'))));
        assert!(parse(r"a \middle| b").is_err());
        let n = parse(r"\begin{array}{|c|c|} \hline 1 & 2 \\[2pt] \hline 3 & 4 \\ \hline \end{array}").unwrap();
        let Node::Array(a) = &n[0] else { panic!() };
        assert_eq!(a.vlines, vec![0, 1, 2]);
        assert_eq!(a.hlines, vec![0, 1, 2]);
        assert_eq!(a.rows.len(), 2);
        assert!((a.row_gaps[0] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn variants_are_scoped() {
        let n = parse(r"\mathbf{x}y").unwrap();
        assert!(matches!(&n[0], Node::Row(v) if matches!(v[0], Node::Symbol { variant: Variant::Bold, .. })));
        assert!(matches!(
            &n[1],
            Node::Symbol {
                variant: Variant::Normal,
                ..
            }
        ));
    }

    #[test]
    fn colors_and_dimensions() {
        let n = parse(r"\textcolor{red}{x} \color{#00ff00} y").unwrap();
        assert!(matches!(
            &n[0],
            Node::Color {
                color: Color(255, 0, 0, 255),
                ..
            }
        ));
        assert!(matches!(
            &n[1],
            Node::Color {
                color: Color(0, 255, 0, 255),
                ..
            }
        ));
        assert!(parse(r"\color{nosuchcolor} x").is_err());
        let n = parse(r"a \hspace{1em} b \kern 2pt c").unwrap();
        assert!(matches!(&n[1], Node::Space { mu } if (*mu - 18.0).abs() < 1e-4));
        assert!(matches!(&n[3], Node::Space { mu } if (*mu - 3.6).abs() < 1e-4));
    }

    #[test]
    fn unicode_input() {
        let n = parse("α ≤ ∑").unwrap();
        assert!(matches!(
            &n[0],
            Node::Symbol {
                ch: 'α',
                atom: AtomType::Ord,
                ..
            }
        ));
        assert!(matches!(
            &n[1],
            Node::Symbol {
                ch: '≤',
                atom: AtomType::Rel,
                ..
            }
        ));
        assert!(matches!(&n[2], Node::BigOp { ch: '∑', .. }));
    }

    #[test]
    fn macros_expand_before_parsing() {
        let n = parse(r"\newcommand{\R}{\mathbb{R}} x \in \R").unwrap();
        assert_eq!(n.len(), 3);
        assert!(matches!(&n[2], Node::Row(v) if matches!(v[0], Node::Symbol { variant: Variant::DoubleStruck, .. })));
    }

    #[test]
    fn unknown_command_is_an_error() {
        let e = parse(r"\foo").unwrap_err();
        assert!(matches!(e, Error::Parse { .. }));
    }
}
