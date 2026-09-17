//! Recursive-descent parser: TeX math source -> `Node` tree.
//!
//! The supported command set follows KaTeX's function list. Unknown commands
//! are hard errors rather than silently dropped, because a formula that
//! renders with a missing piece is worse than one that fails loudly.

use crate::ast::*;
use crate::error::{Error, Result};
use crate::lexer::{Lexer, Tok};
use crate::symbols;

pub fn parse(src: &str) -> Result<Vec<Node>> {
    let mut p = Parser {
        lx: Lexer::new(src),
        variant: Variant::Normal,
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
}

/// Why `parse_list` stopped. The stopping token is left unconsumed.
fn is_stop(tok: &Tok<'_>) -> bool {
    matches!(
        tok,
        Tok::Eof | Tok::RBrace | Tok::Amp | Tok::Cmd("\\") | Tok::Cmd("right") | Tok::Cmd("end")
    )
}

impl<'a> Parser<'a> {
    /// Parses atoms until a group/row/environment terminator.
    fn parse_list(&mut self) -> Result<Vec<Node>> {
        let mut out = Vec::new();
        loop {
            if is_stop(self.lx.peek()?) {
                return Ok(out);
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
                    rule: true,
                    style,
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
                let frac = Node::Frac {
                    num: Box::new(num),
                    den: Box::new(den),
                    rule: false,
                    style,
                };
                Ok(Node::LeftRight {
                    left: Some('('),
                    body: vec![frac],
                    right: Some(')'),
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
                let body = self.parse_list()?;
                match self.lx.advance()? {
                    Tok::Cmd("right") => {}
                    _ => return Err(Error::parse(pos, "\\left without matching \\right")),
                }
                let right = self.parse_delimiter(pos)?;
                Ok(Node::LeftRight { left, body, right })
            }
            "right" => Err(Error::parse(pos, "\\right without matching \\left")),
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
            | "mathnormal" | "mathscr" | "pmb" => {
                let v = match name {
                    "mathbf" | "pmb" => Variant::Bold,
                    "mathrm" => Variant::Roman,
                    "mathit" => Variant::Italic,
                    "mathbb" => Variant::DoubleStruck,
                    "mathcal" | "mathscr" => Variant::Script,
                    "mathfrak" => Variant::Fraktur,
                    "mathsf" => Variant::SansSerif,
                    "mathtt" => Variant::Monospace,
                    "boldsymbol" | "bm" => Variant::BoldItalic,
                    _ => Variant::Normal,
                };
                let saved = self.variant;
                self.variant = v;
                let arg = self.parse_arg();
                self.variant = saved;
                arg
            }
            "text" | "textrm" | "textnormal" | "mbox" | "textit" | "textbf" | "textsf" | "texttt" => {
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
                let raw = self.lx.raw_group()?.trim().to_string();
                let default = if name == "operatorname" {
                    Limits::NoLimits
                } else {
                    Limits::Default
                };
                let limits = self.parse_limits_modifier(default)?;
                Ok(Node::FnName { name: raw, limits })
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
            "phantom" | "hphantom" | "vphantom" => Ok(Node::Phantom(Box::new(self.parse_arg()?))),
            "limits" | "nolimits" => Ok(Node::Row(vec![])),
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
            "\\" => Err(Error::parse(pos, "\\\\ outside of an array")),
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

    fn parse_environment(&mut self, env: &str, pos: usize) -> Result<Node> {
        let (cols, cell_style, left, right): (Vec<ColAlign>, MathStyle, Delim, Delim) = match env {
            "matrix" | "pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix" | "smallmatrix" => {
                let (l, r) = match env {
                    "pmatrix" => (Some('('), Some(')')),
                    "bmatrix" => (Some('['), Some(']')),
                    "Bmatrix" => (Some('{'), Some('}')),
                    "vmatrix" => (Some('|'), Some('|')),
                    "Vmatrix" => (Some('‖'), Some('‖')),
                    _ => (None, None),
                };
                let style = if env == "smallmatrix" { MathStyle::Script } else { MathStyle::Text };
                (vec![], style, l, r)
            }
            "cases" | "dcases" => (
                vec![ColAlign::Left, ColAlign::Left],
                if env == "dcases" { MathStyle::Display } else { MathStyle::Text },
                Some('{'),
                None,
            ),
            "rcases" => (vec![ColAlign::Left, ColAlign::Left], MathStyle::Text, None, Some('}')),
            "array" => {
                let spec = self.lx.raw_group()?;
                let cols = spec
                    .chars()
                    .filter_map(|c| match c {
                        'l' => Some(ColAlign::Left),
                        'c' => Some(ColAlign::Center),
                        'r' => Some(ColAlign::Right),
                        _ => None,
                    })
                    .collect();
                (cols, MathStyle::Text, None, None)
            }
            "aligned" | "align" | "align*" | "split" | "alignat" | "alignat*" | "alignedat" | "flalign" | "flalign*" => {
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
        loop {
            let mut cell = self.parse_list()?;
            // amsmath starts every even column with `{}` so that a leading
            // relation (`a &= b`) still receives its left-hand spacing.
            if is_aligned && rows.last().unwrap().len() % 2 == 1 {
                cell.insert(0, Node::Row(vec![]));
            }
            rows.last_mut().unwrap().push(cell);
            match self.lx.advance()? {
                Tok::Amp => {}
                Tok::Cmd("\\") => {
                    // Optional row spacing `\\[2pt]` is accepted and ignored.
                    if matches!(self.lx.peek()?, Tok::Char('[')) {
                        while !matches!(self.lx.advance()?, Tok::Char(']') | Tok::Eof) {}
                    }
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
        // A trailing `\\` leaves an empty last row; drop it.
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
            cols
        };
        let array = Node::Array { rows, cols, cell_style };
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
        assert!(matches!(&n[0], Node::Frac { rule: true, .. }));
        assert!(matches!(&n[1], Node::Sqrt { index: Some(_), .. }));
    }

    #[test]
    fn left_right_and_matrix() {
        let n = parse(r"\left( \begin{matrix} a & b \\ c & d \end{matrix} \right)").unwrap();
        let Node::LeftRight { left, body, right } = &n[0] else {
            panic!("{n:?}")
        };
        assert_eq!((*left, *right), (Some('('), Some(')')));
        let Node::Array { rows, cols, .. } = &body[0] else { panic!() };
        assert_eq!(rows.len(), 2);
        assert_eq!(cols.len(), 2);
        assert!(parse(r"\left( x").is_err());
        assert!(parse(r"\begin{matrix} a \end{pmatrix}").is_err());
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
    fn unknown_command_is_an_error() {
        let e = parse(r"\foo").unwrap_err();
        assert!(matches!(e, Error::Parse { .. }));
    }
}
