//! Text mode: the inside of `\text{}`, `\mbox{}` and their relatives.
//!
//! Real documents put far more than plain words in a text box: math islands
//! (`\mbox{if $x > 0$}`), font switches (`{\bf A}`, `\textit{}`), sizes
//! (`\mbox{\small note}`), accents (`\'e`), escapes (`\%`) and TeX's dash and
//! quote ligatures. All of it is parsed here into ordinary nodes: runs of
//! `Node::Text`, spaces, and math islands as groups, so layout, hit testing
//! and accessibility need nothing new.

use super::{Parser, MAX_DEPTH};
use crate::ast::{MathStyle, Node, Variant};
use crate::error::{Error, Result};
use crate::lexer::{TextTok, Tok};
use crate::symbols;

/// The size commands as KaTeX and LaTeX's 10 pt class define them, relative
/// to `\normalsize`.
pub(super) fn size_factor(name: &str) -> Option<f32> {
    Some(match name {
        "tiny" => 0.5,
        "scriptsize" => 0.7,
        "footnotesize" => 0.8,
        "small" => 0.9,
        "normalsize" => 1.0,
        "large" => 1.2,
        "Large" => 1.44,
        "LARGE" => 1.728,
        "huge" => 2.074,
        "Huge" => 2.488,
        _ => return None,
    })
}

/// Characters text mode names with a command: Latin letters outside ASCII,
/// escapes of TeX's special characters, and typographic symbols.
pub(super) fn text_symbol(name: &str) -> Option<char> {
    Some(match name {
        "AA" => 'Å',
        "aa" => 'å',
        "AE" => 'Æ',
        "ae" => 'æ',
        "OE" => 'Œ',
        "oe" => 'œ',
        "O" => 'Ø',
        "o" => 'ø',
        "ss" => 'ß',
        "L" => 'Ł',
        "l" => 'ł',
        "i" => 'ı',
        "j" => 'ȷ',
        "%" => '%',
        "$" => '$',
        "&" => '&',
        "#" => '#',
        "_" => '_',
        "{" => '{',
        "}" => '}',
        "textbackslash" => '\\',
        "textasciitilde" => '~',
        "textasciicircum" => '^',
        "textbar" => '|',
        "textless" => '<',
        "textgreater" => '>',
        "textendash" => '–',
        "textemdash" => '—',
        "textquoteleft" => '‘',
        "textquoteright" => '’',
        "textquotedblleft" => '“',
        "textquotedblright" => '”',
        "textdegree" => '°',
        "textregistered" => '®',
        "texttrademark" => '™',
        "copyright" | "textcopyright" => '©',
        "pounds" | "textsterling" => '£',
        "S" | "textsection" => '§',
        "P" | "textparagraph" => '¶',
        "dag" | "textdagger" => '†',
        "ddag" | "textdaggerdbl" => '‡',
        "dots" | "ldots" | "textellipsis" => '…',
        "textbullet" => '•',
        "textperiodcentered" => '·',
        "slash" => '/',
        _ => return None,
    })
}

/// The combining mark for a text accent command.
fn accent_mark(name: &str) -> Option<char> {
    Some(match name {
        "'" => '\u{0301}',
        "`" => '\u{0300}',
        "^" => '\u{0302}',
        "\"" => '\u{0308}',
        "~" => '\u{0303}',
        "=" => '\u{0304}',
        "." => '\u{0307}',
        "u" => '\u{0306}',
        "v" => '\u{030C}',
        "H" => '\u{030B}',
        "c" => '\u{0327}',
        "r" => '\u{030A}',
        "k" => '\u{0328}',
        "d" => '\u{0323}',
        "b" => '\u{0331}',
        "t" => '\u{0361}',
        _ => return None,
    })
}

/// Precomposed forms for the accented letters European names and words use.
/// Each table pairs a base letter with its composed form.
fn compose(mark: char, base: char) -> Option<char> {
    let table = match mark {
        '\u{0301}' => "aáeéiíoóuúyýAÁEÉIÍOÓUÚYÝcćCĆnńNŃsśSŚzźZŹlĺLĹrŕRŔgǵ",
        '\u{0300}' => "aàeèiìoòuùAÀEÈIÌOÒUÙ",
        '\u{0302}' => "aâeêiîoôuûAÂEÊIÎOÔUÛcĉgĝhĥjĵsŝwŵyŷ",
        '\u{0308}' => "aäeëiïoöuüyÿAÄEËIÏOÖUÜYŸ",
        '\u{0303}' => "aãnñoõAÃNÑOÕiĩuũ",
        '\u{0304}' => "aāeēiīoōuūAĀEĒIĪOŌUŪ",
        '\u{0307}' => "zżZŻeėEĖcċgġIİ",
        '\u{0306}' => "aăgğuŭAĂGĞeĕoŏiĭ",
        '\u{030C}' => "cčsšzžrřeěnňdďtťCČSŠZŽRŘEĚNŇDĎTŤ",
        '\u{030B}' => "oőuűOŐUŰ",
        '\u{0327}' => "cçCÇsşSŞtţgģkķlļnņrŗ",
        '\u{030A}' => "aåuůAÅUŮ",
        '\u{0328}' => "aąeęAĄEĘiįuų",
        _ => return None,
    };
    let base = match base {
        'ı' => 'i',
        'ȷ' => 'j',
        c => c,
    };
    let chars: Vec<char> = table.chars().collect();
    chars.chunks(2).find(|p| p[0] == base).map(|p| p[1])
}

/// TeX's text ligatures for dashes and quotes.
fn ligatures(run: &str) -> String {
    run.replace("---", "—")
        .replace("--", "–")
        .replace("``", "“")
        .replace("''", "”")
        .replace('`', "‘")
        .replace('\'', "’")
}

#[derive(Clone, Copy)]
struct State {
    variant: Variant,
    size: f32,
    boldmath: bool,
}

impl State {
    fn bold(mut self) -> Self {
        self.variant = match self.variant {
            Variant::Italic | Variant::BoldItalic => Variant::BoldItalic,
            _ => Variant::Bold,
        };
        self
    }
    fn italic(mut self) -> Self {
        self.variant = match self.variant {
            Variant::Bold | Variant::BoldItalic => Variant::BoldItalic,
            _ => Variant::Italic,
        };
        self
    }
    fn with(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }
}

/// Collects a text group's output, merging characters set in the same font
/// and size into one run.
struct Out {
    nodes: Vec<Node>,
    run: String,
    run_variant: Variant,
    run_size: f32,
}

impl Out {
    fn push_char(&mut self, c: char, st: State) {
        if !self.run.is_empty() && (self.run_variant != st.variant || self.run_size != st.size) {
            self.flush();
        }
        if c == ' ' && self.run.ends_with(' ') {
            return;
        }
        self.run_variant = st.variant;
        self.run_size = st.size;
        self.run.push(c);
    }

    fn push_node(&mut self, node: Node, st: State) {
        self.flush();
        self.nodes.push(sized(node, st.size));
    }

    fn flush(&mut self) {
        if self.run.is_empty() {
            return;
        }
        let text = ligatures(&std::mem::take(&mut self.run));
        let node = Node::Text {
            text,
            variant: self.run_variant,
        };
        self.nodes.push(sized(node, self.run_size));
    }
}

fn sized(node: Node, size: f32) -> Node {
    if size == 1.0 {
        node
    } else {
        Node::Size {
            factor: size,
            body: vec![node],
        }
    }
}

impl<'a> Parser<'a> {
    /// The argument of `\text` and its relatives, set in `variant`.
    pub(super) fn parse_text_arg(&mut self, variant: Variant, pos: usize) -> Result<Node> {
        let st = State {
            variant,
            size: 1.0,
            boldmath: false,
        };
        let mut out = Out {
            nodes: Vec::new(),
            run: String::new(),
            run_variant: variant,
            run_size: 1.0,
        };
        // TeX takes a single token as the argument when there is no group.
        match self.lx.peek()? {
            Tok::LBrace => {
                self.lx.advance()?;
                self.text_body(st, &mut out, pos)?;
            }
            Tok::Char(_) | Tok::Cmd(_) => match self.lx.text_tok()? {
                TextTok::Char(c) => out.push_char(c, st),
                TextTok::Cmd(name) => {
                    let mut st = st;
                    self.text_command(name, &mut st, &mut out, pos)?;
                }
                _ => unreachable!("peeked a character or command"),
            },
            t => return Err(Error::parse(pos, format!("expected text, found {t:?}"))),
        }
        out.flush();
        Ok(match out.nodes.len() {
            0 => Node::Text {
                text: String::new(),
                variant,
            },
            1 => out.nodes.pop().unwrap(),
            _ => Node::Row(out.nodes),
        })
    }

    /// A single text command met in math mode, set upright as text.
    pub(super) fn parse_text_arg_from_command(&mut self, name: &'a str, pos: usize) -> Result<Node> {
        let mut st = State {
            variant: Variant::Roman,
            size: 1.0,
            boldmath: false,
        };
        let mut out = Out {
            nodes: Vec::new(),
            run: String::new(),
            run_variant: Variant::Roman,
            run_size: 1.0,
        };
        self.text_command(name, &mut st, &mut out, pos)?;
        out.flush();
        Ok(Node::row(out.nodes))
    }

    /// Text up to the closing brace of the current group.
    fn text_body(&mut self, st: State, out: &mut Out, pos: usize) -> Result<()> {
        if self.depth > MAX_DEPTH {
            return Err(Error::parse(pos, format!("nesting deeper than {MAX_DEPTH} levels")));
        }
        self.depth += 1;
        let r = self.text_body_inner(st, out, pos);
        self.depth -= 1;
        r
    }

    fn text_body_inner(&mut self, mut st: State, out: &mut Out, pos: usize) -> Result<()> {
        loop {
            match self.lx.text_tok()? {
                TextTok::Eof => return Err(Error::parse(pos, "missing `}` after text")),
                TextTok::RBrace => return Ok(()),
                TextTok::LBrace => self.text_body(st, out, pos)?,
                TextTok::Char('$') => {
                    let island = self.text_math(st, Tok::Char('$'), pos)?;
                    out.push_node(island, st);
                }
                TextTok::Char(c) if c.is_whitespace() || c == '~' => out.push_char(' ', st),
                TextTok::Char(c) => out.push_char(c, st),
                TextTok::Cmd(name) => self.text_command(name, &mut st, out, pos)?,
            }
            self.count_node(pos)?;
        }
    }

    /// `$...$` or `\(...\)` inside text: math at text style, as one group so
    /// it is spaced as a unit against the words around it.
    fn text_math(&mut self, st: State, close: Tok<'static>, pos: usize) -> Result<Node> {
        let saved = (self.variant, self.text_math_close.take());
        if st.boldmath {
            self.variant = Variant::BoldItalic;
        }
        self.text_math_close = Some(close.clone());
        let body = self.parse_list();
        self.variant = saved.0;
        self.text_math_close = saved.1;
        let body = body?;
        if self.lx.advance()? != close {
            return Err(Error::parse(pos, "math in text is not closed"));
        }
        Ok(Node::Row(vec![Node::Style {
            style: MathStyle::Text,
            body,
        }]))
    }

    fn text_command(&mut self, name: &'a str, st: &mut State, out: &mut Out, pos: usize) -> Result<()> {
        if let Some(c) = text_symbol(name) {
            out.push_char(c, *st);
            return Ok(());
        }
        if let Some(f) = size_factor(name) {
            st.size = f;
            return Ok(());
        }
        if let Some(mark) = accent_mark(name) {
            let base = self.text_accent_base(pos)?;
            match base.and_then(|b| compose(mark, b)) {
                Some(c) => out.push_char(c, *st),
                None => {
                    if let Some(b) = base {
                        out.push_char(b, *st);
                    }
                    out.push_char(mark, *st);
                }
            }
            return Ok(());
        }
        match name {
            " " | "\t" | "\n" | "\r" | "space" | "nobreakspace" => out.push_char(' ', *st),
            "TeX" | "LaTeX" | "KaTeX" => name.chars().for_each(|c| out.push_char(c, *st)),
            "\\" | "newline" | "linebreak" | "par" | "noindent" | "nolinebreak" | "nobreak" | "relax" | "hfill" | "hfil" | "hss"
            | "null" | "unskip" | "-" | "/" | "@" | "sc" | "scshape" | "upshape" | "mdseries" | "rmfamily" | "protect" | "displaystyle"
            | "textstyle" => {}
            "rm" | "normalfont" => st.variant = Variant::Roman,
            "bf" | "bfseries" => *st = st.bold(),
            "it" | "itshape" | "sl" | "slshape" => *st = st.italic(),
            "em" => {
                *st = if st.variant == Variant::Italic {
                    st.with(Variant::Roman)
                } else {
                    st.italic()
                }
            }
            "sf" | "sffamily" => st.variant = Variant::SansSerif,
            "tt" | "ttfamily" => st.variant = Variant::Monospace,
            "boldmath" => st.boldmath = true,
            "unboldmath" => st.boldmath = false,
            "textbf" => self.text_group(st.bold(), out, pos)?,
            "textit" | "textsl" | "emph" => {
                let s = if name == "emph" && st.variant == Variant::Italic {
                    st.with(Variant::Roman)
                } else {
                    st.italic()
                };
                self.text_group(s, out, pos)?
            }
            "textrm" | "textup" | "textnormal" | "textmd" => self.text_group(st.with(Variant::Roman), out, pos)?,
            "textsf" => self.text_group(st.with(Variant::SansSerif), out, pos)?,
            "texttt" => self.text_group(st.with(Variant::Monospace), out, pos)?,
            "text" | "mbox" | "hbox" | "textsc" | "makebox" | "fbox" => self.text_group(*st, out, pos)?,
            "(" => {
                let island = self.text_math(*st, Tok::Cmd(")"), pos)?;
                out.push_node(island, *st);
            }
            _ if symbols::SPACES.iter().any(|(n, _)| *n == name) => {
                let mu = symbols::SPACES.iter().find(|(n, _)| *n == name).unwrap().1;
                out.push_node(Node::Space { mu }, *st);
            }
            // Anything else is read as math: `\mbox{\alpha-decay}`, `\strut`,
            // `\hspace{1em}` and `\color` work as they would around the text.
            // A command math mode does not know is an error there too.
            _ => {
                let node = self.parse_command(name, pos)?;
                out.push_node(node, *st);
            }
        }
        Ok(())
    }

    /// `{...}` after a text command such as `\textbf`, in the given state.
    fn text_group(&mut self, st: State, out: &mut Out, pos: usize) -> Result<()> {
        loop {
            match self.lx.text_tok()? {
                TextTok::Char(c) if c.is_whitespace() => continue,
                TextTok::LBrace => return self.text_body(st, out, pos),
                TextTok::Char(c) => {
                    out.push_char(c, st);
                    return Ok(());
                }
                _ => return Err(Error::parse(pos, "expected a group after a text command")),
            }
        }
    }

    /// The letter under a text accent: `\'e`, `\'{e}`, `\'{\i}` or `\c C`.
    fn text_accent_base(&mut self, pos: usize) -> Result<Option<char>> {
        let mut braced = false;
        loop {
            match self.lx.text_tok()? {
                TextTok::LBrace if !braced => braced = true,
                TextTok::Char(c) if c.is_whitespace() => continue,
                TextTok::RBrace if braced => return Ok(None),
                TextTok::Char(c) => return self.close_accent(braced, Some(c), pos),
                TextTok::Cmd(n) => {
                    let c = text_symbol(n);
                    return self.close_accent(braced, c, pos);
                }
                _ => return Err(Error::parse(pos, "accent without a letter")),
            }
        }
    }

    fn close_accent(&mut self, braced: bool, c: Option<char>, pos: usize) -> Result<Option<char>> {
        if braced && self.lx.text_tok()? != TextTok::RBrace {
            return Err(Error::parse(pos, "an accent takes one letter"));
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Node, Variant};
    use crate::parse;

    /// The text runs of a parsed formula, in order, with their variants.
    fn runs(tex: &str) -> Vec<(String, Variant)> {
        fn walk(n: &Node, out: &mut Vec<(String, Variant)>) {
            match n {
                Node::Text { text, variant } => out.push((text.clone(), *variant)),
                Node::Row(v) | Node::Style { body: v, .. } | Node::Size { body: v, .. } => v.iter().for_each(|c| walk(c, out)),
                _ => {}
            }
        }
        let mut out = Vec::new();
        parse(tex).unwrap().iter().for_each(|n| walk(n, &mut out));
        out
    }

    fn text(tex: &str) -> String {
        runs(tex).into_iter().map(|r| r.0).collect()
    }

    #[test]
    fn math_islands_are_math() {
        let nodes = parse(r"\text{if $x > 0$ then}").unwrap();
        let Node::Row(parts) = &nodes[0] else { panic!("{nodes:?}") };
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[0], Node::Text { text, .. } if text == "if "));
        assert!(matches!(&parts[1], Node::Row(v) if matches!(&v[0], Node::Style { .. })));
        assert!(matches!(&parts[2], Node::Text { text, .. } if text == " then"));
        assert!(parse(r"\mbox{\(a\)}").is_ok());
        assert!(parse(r"\text{$x").is_err());
    }

    #[test]
    fn font_switches_are_scoped() {
        assert_eq!(
            runs(r"\mbox{a {\bf b} c}"),
            [
                ("a ".into(), Variant::Roman),
                ("b".into(), Variant::Bold),
                (" c".into(), Variant::Roman)
            ]
        );
        assert_eq!(runs(r"\textbf{\textit{x}}")[0].1, Variant::BoldItalic);
        assert_eq!(runs(r"\emph{a \emph{b}}")[1].1, Variant::Roman);
    }

    #[test]
    fn sizes_scale_text_and_math() {
        let nodes = parse(r"\mbox{\small note}").unwrap();
        assert!(matches!(&nodes[0], Node::Size { factor, .. } if *factor == 0.9));
        let nodes = parse(r"a {\Large b}").unwrap();
        assert!(matches!(&nodes[1], Node::Row(v) if matches!(&v[0], Node::Size { factor, .. } if *factor == 1.44)));
    }

    #[test]
    fn accents_escapes_and_ligatures() {
        assert_eq!(text(r#"\text{Erd\H{o}s and G\"odel, na\"{\i}ve}"#), "Erdős and Gödel, naïve");
        assert_eq!(text(r#"\text{\c{c}a, \v{S}koda, \AA ngstr\"om}"#), "ça, Škoda, Ångström");
        assert_eq!(text(r"\text{50\% of \$5 \& more}"), "50% of $5 & more");
        assert_eq!(text(r"\text{pages 1--2 --- ``quoted'' don't}"), "pages 1–2 — “quoted” don’t");
        assert_eq!(text(r"\text{a   b}"), "a b");
        assert_eq!(text(r"\text{\LaTeX\ and \TeX}"), "LaTeX and TeX");
    }

    #[test]
    fn math_commands_in_text_are_read_as_math() {
        assert!(parse(r"\mbox{\alpha-decay}").is_ok());
        assert!(parse(r"\mbox{\boldmath $e$}").is_ok());
        assert!(parse(r"\text{a\hspace{1em}b}").is_ok());
        assert!(parse(r"\text{\nosuchcommand}").is_err());
    }

    #[test]
    fn text_letters_in_math() {
        assert_eq!(text(r"1.3\AA"), "Å");
        assert_eq!(text(r"\c C"), "Ç");
    }
}
