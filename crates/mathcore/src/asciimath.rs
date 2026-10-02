//! AsciiMath input (asciimath.org), translated to TeX.
//!
//! AsciiMath is what people type when they do not know TeX: `sum_(i=1)^n i^2`,
//! `sqrt(x)/2`, `[[a,b],[c,d]]`. The grammar is ASCIIMathML's:
//!
//! ```text
//! E ::= I E | I/I E          expression: a sequence, `/` makes fractions
//! I ::= S | S_S | S^S | S_S^S   intermediate: scripts
//! S ::= v | lEr | uS | bSS   simple: symbol, bracketed, unary, binary
//! ```
//!
//! Brackets around a fraction's operands, a root's index or a script are
//! dropped, as AsciiMath does: `(a+b)/2` is `\frac{a+b}{2}`.

use crate::error::{Error, Result};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// A symbol: its TeX.
    Const,
    /// One argument: `sqrt`, `hat`, `bb`.
    Unary,
    /// Two arguments: `frac`, `root`, `color`.
    Binary,
    LeftBracket,
    RightBracket,
    /// `|`, `||`: left or right depending on position.
    LeftRight,
    /// `sin`, `log`: a function name, which takes the next simple expression.
    Func,
    /// `"text"` or `text(...)`.
    Text,
    Infix,
}

struct Sym {
    input: &'static str,
    tex: &'static str,
    kind: Kind,
}

const fn s(input: &'static str, tex: &'static str, kind: Kind) -> Sym {
    Sym { input, tex, kind }
}

use Kind::*;

/// The symbol table, after ASCIIMathML.js. Longest match wins.
static SYMBOLS: &[Sym] = &[
    // Greek
    s("alpha", r"\alpha", Const),
    s("beta", r"\beta", Const),
    s("chi", r"\chi", Const),
    s("delta", r"\delta", Const),
    s("Delta", r"\Delta", Const),
    s("epsi", r"\epsilon", Const),
    s("epsilon", r"\epsilon", Const),
    s("varepsilon", r"\varepsilon", Const),
    s("eta", r"\eta", Const),
    s("gamma", r"\gamma", Const),
    s("Gamma", r"\Gamma", Const),
    s("iota", r"\iota", Const),
    s("kappa", r"\kappa", Const),
    s("lambda", r"\lambda", Const),
    s("Lambda", r"\Lambda", Const),
    s("lamda", r"\lambda", Const),
    s("Lamda", r"\Lambda", Const),
    s("mu", r"\mu", Const),
    s("nu", r"\nu", Const),
    s("omega", r"\omega", Const),
    s("Omega", r"\Omega", Const),
    s("phi", r"\phi", Const),
    s("varphi", r"\varphi", Const),
    s("Phi", r"\Phi", Const),
    s("pi", r"\pi", Const),
    s("Pi", r"\Pi", Const),
    s("psi", r"\psi", Const),
    s("Psi", r"\Psi", Const),
    s("rho", r"\rho", Const),
    s("sigma", r"\sigma", Const),
    s("Sigma", r"\Sigma", Const),
    s("tau", r"\tau", Const),
    s("theta", r"\theta", Const),
    s("vartheta", r"\vartheta", Const),
    s("Theta", r"\Theta", Const),
    s("upsilon", r"\upsilon", Const),
    s("xi", r"\xi", Const),
    s("Xi", r"\Xi", Const),
    s("zeta", r"\zeta", Const),
    // Operators
    s("+", "+", Const),
    s("-", "-", Const),
    s("*", r"\cdot", Const),
    s("**", r"\ast", Const),
    s("***", r"\star", Const),
    s("//", "/", Const),
    s("\\\\", r"\backslash", Const),
    s("setminus", r"\setminus", Const),
    s("xx", r"\times", Const),
    s("|><", r"\ltimes", Const),
    s("><|", r"\rtimes", Const),
    s("|><|", r"\bowtie", Const),
    s("-:", r"\div", Const),
    s("divide", r"\div", Const),
    s("@", r"\circ", Const),
    s("o+", r"\oplus", Const),
    s("ox", r"\otimes", Const),
    s("o.", r"\odot", Const),
    s("sum", r"\sum", Const),
    s("prod", r"\prod", Const),
    s("^^", r"\wedge", Const),
    s("^^^", r"\bigwedge", Const),
    s("vv", r"\vee", Const),
    s("vvv", r"\bigvee", Const),
    s("nn", r"\cap", Const),
    s("nnn", r"\bigcap", Const),
    s("uu", r"\cup", Const),
    s("uuu", r"\bigcup", Const),
    // Relations
    s("=", "=", Const),
    s("!=", r"\ne", Const),
    s(":=", r"\coloneqq", Const),
    s("lt", "<", Const),
    s("<", "<", Const),
    s("gt", ">", Const),
    s(">", ">", Const),
    s("<=", r"\le", Const),
    s("lt=", r"\le", Const),
    s(">=", r"\ge", Const),
    s("gt=", r"\ge", Const),
    s("mlt", r"\ll", Const),
    s("mgt", r"\gg", Const),
    s("-<", r"\prec", Const),
    s("-<=", r"\preceq", Const),
    s(">-", r"\succ", Const),
    s(">-=", r"\succeq", Const),
    s("in", r"\in", Const),
    s("!in", r"\notin", Const),
    s("sub", r"\subset", Const),
    s("sup", r"\supset", Const),
    s("sube", r"\subseteq", Const),
    s("supe", r"\supseteq", Const),
    s("-=", r"\equiv", Const),
    s("~=", r"\cong", Const),
    s("~~", r"\approx", Const),
    s("~", r"\sim", Const),
    s("prop", r"\propto", Const),
    // Logic
    s("and", r"\text{ and }", Const),
    s("or", r"\text{ or }", Const),
    s("not", r"\neg", Const),
    s("=>", r"\implies", Const),
    s("if", r"\text{ if }", Const),
    s("<=>", r"\iff", Const),
    s("AA", r"\forall", Const),
    s("EE", r"\exists", Const),
    s("_|_", r"\bot", Const),
    s("TT", r"\top", Const),
    s("|--", r"\vdash", Const),
    s("|==", r"\models", Const),
    // Brackets
    s("(", "(", LeftBracket),
    s(")", ")", RightBracket),
    s("[", "[", LeftBracket),
    s("]", "]", RightBracket),
    s("{", r"\{", LeftBracket),
    s("}", r"\}", RightBracket),
    s("(:", r"\langle", LeftBracket),
    s(":)", r"\rangle", RightBracket),
    s("<<", r"\langle", LeftBracket),
    s(">>", r"\rangle", RightBracket),
    s("{:", ".", LeftBracket),
    s(":}", ".", RightBracket),
    s("|", "|", LeftRight),
    s("||", r"\|", LeftRight),
    // Miscellaneous
    s("int", r"\int", Const),
    s("oint", r"\oint", Const),
    s("del", r"\partial", Const),
    s("grad", r"\nabla", Const),
    s("+-", r"\pm", Const),
    s("-+", r"\mp", Const),
    s("O/", r"\emptyset", Const),
    s("oo", r"\infty", Const),
    s("aleph", r"\aleph", Const),
    s("...", r"\ldots", Const),
    s(":.", r"\therefore", Const),
    s(":'", r"\because", Const),
    s("/_", r"\angle", Const),
    s("/_\\", r"\triangle", Const),
    s("'", "'", Const),
    s("\\ ", r"\ ", Const),
    s("frown", r"\frown", Const),
    s("quad", r"\quad", Const),
    s("qquad", r"\qquad", Const),
    s("cdots", r"\cdots", Const),
    s("vdots", r"\vdots", Const),
    s("ddots", r"\ddots", Const),
    s("diamond", r"\diamond", Const),
    s("square", r"\square", Const),
    s("|__", r"\lfloor", Const),
    s("__|", r"\rfloor", Const),
    s("|~", r"\lceil", Const),
    s("~|", r"\rceil", Const),
    s("CC", r"\mathbb{C}", Const),
    s("NN", r"\mathbb{N}", Const),
    s("QQ", r"\mathbb{Q}", Const),
    s("RR", r"\mathbb{R}", Const),
    s("ZZ", r"\mathbb{Z}", Const),
    // Arrows
    s("uarr", r"\uparrow", Const),
    s("darr", r"\downarrow", Const),
    s("rarr", r"\rightarrow", Const),
    s("->", r"\to", Const),
    s(">->", r"\rightarrowtail", Const),
    s("->>", r"\twoheadrightarrow", Const),
    s(">->>", r"\twoheadrightarrowtail", Const),
    s("|->", r"\mapsto", Const),
    s("larr", r"\leftarrow", Const),
    s("harr", r"\leftrightarrow", Const),
    s("rArr", r"\Rightarrow", Const),
    s("lArr", r"\Leftarrow", Const),
    s("hArr", r"\Leftrightarrow", Const),
    // Functions
    s("sin", r"\sin", Func),
    s("cos", r"\cos", Func),
    s("tan", r"\tan", Func),
    s("sec", r"\sec", Func),
    s("csc", r"\csc", Func),
    s("cot", r"\cot", Func),
    s("arcsin", r"\arcsin", Func),
    s("arccos", r"\arccos", Func),
    s("arctan", r"\arctan", Func),
    s("sinh", r"\sinh", Func),
    s("cosh", r"\cosh", Func),
    s("tanh", r"\tanh", Func),
    s("sech", r"\operatorname{sech}", Func),
    s("csch", r"\operatorname{csch}", Func),
    s("coth", r"\coth", Func),
    s("exp", r"\exp", Func),
    s("log", r"\log", Func),
    s("ln", r"\ln", Func),
    s("det", r"\det", Func),
    s("dim", r"\dim", Func),
    s("mod", r"\bmod", Const),
    s("gcd", r"\gcd", Func),
    s("lcm", r"\operatorname{lcm}", Func),
    s("lub", r"\operatorname{lub}", Func),
    s("glb", r"\operatorname{glb}", Func),
    s("min", r"\min", Const),
    s("max", r"\max", Const),
    s("lim", r"\lim", Const),
    s("Lim", r"\operatorname*{Lim}", Const),
    s("f", "f", Func),
    s("g", "g", Func),
    // Unary
    s("sqrt", r"\sqrt", Unary),
    s("abs", "abs", Unary),
    s("floor", "floor", Unary),
    s("ceil", "ceil", Unary),
    s("norm", "norm", Unary),
    s("hat", r"\hat", Unary),
    s("bar", r"\overline", Unary),
    s("overline", r"\overline", Unary),
    s("vec", r"\vec", Unary),
    s("tilde", r"\tilde", Unary),
    s("dot", r"\dot", Unary),
    s("ddot", r"\ddot", Unary),
    s("ul", r"\underline", Unary),
    s("underline", r"\underline", Unary),
    s("ubrace", r"\underbrace", Unary),
    s("underbrace", r"\underbrace", Unary),
    s("obrace", r"\overbrace", Unary),
    s("overbrace", r"\overbrace", Unary),
    s("cancel", r"\cancel", Unary),
    s("bb", r"\mathbf", Unary),
    s("mathbf", r"\mathbf", Unary),
    s("sf", r"\mathsf", Unary),
    s("mathsf", r"\mathsf", Unary),
    s("bbb", r"\mathbb", Unary),
    s("mathbb", r"\mathbb", Unary),
    s("cc", r"\mathcal", Unary),
    s("mathcal", r"\mathcal", Unary),
    s("tt", r"\mathtt", Unary),
    s("mathtt", r"\mathtt", Unary),
    s("fr", r"\mathfrak", Unary),
    s("mathfrak", r"\mathfrak", Unary),
    s("rm", r"\mathrm", Unary),
    s("text", r"\text", Text),
    s("mbox", r"\text", Text),
    // Binary
    s("frac", r"\frac", Binary),
    s("root", r"\sqrt", Binary),
    s("stackrel", r"\stackrel", Binary),
    s("overset", r"\overset", Binary),
    s("underset", r"\underset", Binary),
    s("color", r"\textcolor", Binary),
    s("/", "/", Infix),
];

/// Translates AsciiMath to TeX for the parser.
pub fn to_tex(src: &str) -> Result<String> {
    let mut p = P { src, pos: 0, depth: 0 };
    let mut out = String::new();
    while p.skip_ws() < src.len() {
        let before = p.pos;
        let e = p.expression()?;
        out.push_str(&e);
        if p.pos == before {
            // A stray right bracket: emit it as itself and go on, as AsciiMath does.
            let (sym, len) = p.lookup().ok_or_else(|| Error::parse(p.pos, "unexpected input"))?;
            p.pos += len;
            out.push_str(&brace_tex(sym.tex));
        }
    }
    Ok(out)
}

struct P<'s> {
    src: &'s str,
    pos: usize,
    depth: usize,
}

/// A bracket's TeX that is safe in `\left` and `\right`.
fn brace_tex(t: &str) -> String {
    match t {
        "{" => r"\{".into(),
        "}" => r"\}".into(),
        _ => t.into(),
    }
}

/// Drops one level of round, square or curly brackets: `(a+b)` -> `a+b`.
fn unwrap(tex: &str) -> &str {
    for (l, r) in [
        (r"\left(", r"\right)"),
        (r"\left[", r"\right]"),
        (r"\left\{", r"\right\}"),
        (r"\left.", r"\right."),
    ] {
        if let Some(inner) = tex.strip_prefix(l).and_then(|t| t.strip_suffix(r)) {
            // Only if these brackets enclose everything.
            if balanced(inner) {
                return inner;
            }
        }
    }
    tex
}

fn balanced(t: &str) -> bool {
    let mut depth = 0i32;
    let mut i = 0;
    while let Some(k) = t[i..].find(['\\']) {
        let rest = &t[i + k..];
        if rest.starts_with(r"\left") {
            depth += 1;
        } else if rest.starts_with(r"\right") {
            depth -= 1;
            if depth < 0 {
                return false;
            }
        }
        i += k + 1;
    }
    depth == 0
}

const MAX_DEPTH: usize = 64;

impl P<'_> {
    fn skip_ws(&mut self) -> usize {
        while let Some(c) = self.src[self.pos..].chars().next().filter(|c| c.is_whitespace()) {
            self.pos += c.len_utf8();
        }
        self.pos
    }

    /// The longest symbol at the cursor, and its byte length.
    fn lookup(&self) -> Option<(&'static Sym, usize)> {
        let rest = &self.src[self.pos..];
        SYMBOLS
            .iter()
            .filter(|s| rest.starts_with(s.input))
            .max_by_key(|s| s.input.len())
            .map(|s| (s, s.input.len()))
    }

    fn peek_kind(&mut self) -> Option<Kind> {
        self.skip_ws();
        self.lookup().map(|(s, _)| s.kind)
    }

    /// E: a run of intermediates, with `/` building fractions. Stops at a
    /// right bracket (left in place for the caller) or the end.
    fn expression(&mut self) -> Result<String> {
        let mut out = String::new();
        loop {
            self.skip_ws();
            if self.pos >= self.src.len() {
                break;
            }
            if let Some(RightBracket) = self.peek_kind() {
                break;
            }
            // `,` and `;` at matrix level are handled by the bracket parser;
            // here they are punctuation.
            let mut term = self.intermediate()?;
            self.skip_ws();
            if self.lookup().is_some_and(|(s, _)| s.kind == Infix) {
                self.pos += 1;
                let den = self.intermediate()?;
                term = format!(r"\frac{{{}}}{{{}}}", unwrap(&term), unwrap(&den));
            }
            if !out.is_empty() && needs_space(&out, &term) {
                out.push(' ');
            }
            out.push_str(&term);
        }
        Ok(out)
    }

    /// I: a simple expression with optional sub- and superscript.
    fn intermediate(&mut self) -> Result<String> {
        let base = self.simple()?;
        let mut out = base;
        self.skip_ws();
        for op in ['_', '^'] {
            self.skip_ws();
            if self.src[self.pos..].starts_with(op) && !self.src[self.pos..].starts_with("^^") {
                self.pos += 1;
                let script = self.simple()?;
                out.push(op);
                out.push('{');
                out.push_str(unwrap(&script));
                out.push('}');
            }
        }
        Ok(out)
    }

    /// S: one symbol, a bracketed expression, or an operator with its
    /// arguments. Every recursion passes through here, so the depth limit
    /// lives here.
    fn simple(&mut self) -> Result<String> {
        if self.depth > MAX_DEPTH {
            return Err(Error::parse(self.pos, format!("nesting deeper than {MAX_DEPTH} levels")));
        }
        self.depth += 1;
        let r = self.simple_inner();
        self.depth -= 1;
        r
    }

    fn simple_inner(&mut self) -> Result<String> {
        self.skip_ws();
        let rest = &self.src[self.pos..];
        if rest.is_empty() {
            return Ok(String::new());
        }
        // Quoted text.
        if let Some(body) = rest.strip_prefix('"') {
            let end = body.find('"').ok_or_else(|| Error::parse(self.pos, "unclosed quote"))?;
            let text = &body[..end];
            self.pos += end + 2;
            return Ok(format!(r"\text{{{}}}", escape_text(text)));
        }
        // Numbers.
        let num_len = rest
            .char_indices()
            .take_while(|(i, c)| c.is_ascii_digit() || (*c == '.' && rest[i + 1..].starts_with(|d: char| d.is_ascii_digit())))
            .map(|(i, c)| i + c.len_utf8())
            .last()
            .unwrap_or(0);
        if num_len > 0 {
            self.pos += num_len;
            return Ok(rest[..num_len].to_string());
        }
        let Some((sym, len)) = self.lookup() else {
            // A plain character: a letter is a variable.
            let c = rest.chars().next().unwrap();
            self.pos += c.len_utf8();
            return Ok(match c {
                '{' | '}' | '%' | '&' | '#' | '$' => format!(r"\{c}"),
                '_' | '^' => String::new(),
                '\\' => r"\backslash".into(),
                _ => c.to_string(),
            });
        };
        // A symbol's name may be the start of a longer word: `inf` is not `in` `f`.
        let word_continues = sym.input.chars().all(|c| c.is_ascii_alphabetic())
            && rest[len..].starts_with(|c: char| c.is_ascii_alphabetic())
            && sym.input.len() == 1;
        if word_continues {
            self.pos += 1;
            return Ok(rest[..1].to_string());
        }
        self.pos += len;
        match sym.kind {
            Const | Infix => Ok(sym.tex.to_string() + if sym.tex.starts_with('\\') { " " } else { "" }),
            Func => {
                // A function applied to a bracketed argument keeps its brackets.
                Ok(format!("{} ", sym.tex))
            }
            LeftBracket => self.bracketed(sym),
            RightBracket => Ok(brace_tex(sym.tex)),
            LeftRight => self.bars(sym),
            Text => {
                self.skip_ws();
                let rest = &self.src[self.pos..];
                let open = rest.chars().next().unwrap_or(' ');
                let close = match open {
                    '(' => ')',
                    '[' => ']',
                    '{' => '}',
                    _ => return Err(Error::parse(self.pos, "text needs brackets")),
                };
                let end = rest.find(close).ok_or_else(|| Error::parse(self.pos, "unclosed text"))?;
                let text = &rest[1..end];
                self.pos += end + 1;
                Ok(format!(r"\text{{{}}}", escape_text(text)))
            }
            Unary => {
                let arg = self.simple()?;
                let arg = unwrap(&arg).to_string();
                Ok(match sym.tex {
                    "abs" => format!(r"\left|{arg}\right|"),
                    "floor" => format!(r"\left\lfloor {arg}\right\rfloor "),
                    "ceil" => format!(r"\left\lceil {arg}\right\rceil "),
                    "norm" => format!(r"\left\|{arg}\right\|"),
                    t => format!("{t}{{{arg}}}"),
                })
            }
            Binary => {
                let a = self.simple()?;
                let b = self.simple()?;
                let (a, b) = (unwrap(&a).to_string(), unwrap(&b).to_string());
                Ok(match sym.input {
                    "root" => format!(r"\sqrt[{a}]{{{b}}}"),
                    "color" => format!(r"\textcolor{{{}}}{{{b}}}", a.replace(' ', "")),
                    _ => format!("{}{{{a}}}{{{b}}}", sym.tex),
                })
            }
        }
    }

    /// `(…)`, `[…]`, `{: … :}`: an expression between brackets, or a matrix
    /// when its rows are bracketed and comma-separated: `[[a,b],[c,d]]`.
    fn bracketed(&mut self, open: &'static Sym) -> Result<String> {
        let start = self.pos;
        if self.looks_like_matrix() {
            if let Some(m) = self.try_matrix(open)? {
                return Ok(m);
            }
            self.pos = start;
        }
        let inner = self.expression()?;
        self.skip_ws();
        let close = match self.lookup() {
            Some((s, len)) if s.kind == RightBracket || s.kind == LeftRight => {
                self.pos += len;
                brace_tex(s.tex)
            }
            _ => ".".to_string(),
        };
        Ok(format!(r"\left{}{inner}\right{close}", brace_tex(open.tex)))
    }

    /// A cheap scan before trying a matrix: the bracket must open with a
    /// bracketed row followed by `,` and another row. Without it, every
    /// nested bracket would be parsed twice, which is exponential in depth.
    fn looks_like_matrix(&self) -> bool {
        let rest = self.src[self.pos..].trim_start();
        let Some(first) = rest.chars().next() else { return false };
        if !matches!(first, '(' | '[') {
            return false;
        }
        let mut depth = 0i32;
        for (i, c) in rest.char_indices() {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let after = rest[i + 1..].trim_start();
                        return after.starts_with(',') && after[1..].trim_start().starts_with(['(', '[']);
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Rows of the form `(a,b),(c,d)` or `[a,b],[c,d]` inside the outer bracket.
    fn try_matrix(&mut self, open: &'static Sym) -> Result<Option<String>> {
        let mut rows: Vec<Vec<String>> = Vec::new();
        loop {
            self.skip_ws();
            let Some((row_open, len)) = self.lookup() else { return Ok(None) };
            if row_open.kind != LeftBracket || !matches!(row_open.input, "(" | "[") {
                return Ok(None);
            }
            self.pos += len;
            let mut cells = Vec::new();
            loop {
                let cell = self.matrix_cell()?;
                cells.push(cell);
                self.skip_ws();
                if self.src[self.pos..].starts_with(',') {
                    self.pos += 1;
                    continue;
                }
                break;
            }
            match self.lookup() {
                Some((s, len)) if s.kind == RightBracket => self.pos += len,
                _ => return Ok(None),
            }
            rows.push(cells);
            self.skip_ws();
            if self.src[self.pos..].starts_with(',') {
                self.pos += 1;
                continue;
            }
            break;
        }
        let Some((close, len)) = self.lookup().filter(|(s, _)| s.kind == RightBracket) else {
            return Ok(None);
        };
        if rows.len() < 2 && rows.first().is_none_or(|r| r.len() < 2) {
            return Ok(None);
        }
        if rows.iter().any(|r| r.len() != rows[0].len()) {
            return Ok(None);
        }
        self.pos += len;
        let body = rows.iter().map(|r| r.join(" & ")).collect::<Vec<_>>().join(r" \\ ");
        let cols = "c".repeat(rows[0].len());
        Ok(Some(format!(
            r"\left{}\begin{{array}}{{{cols}}} {body} \end{{array}}\right{}",
            brace_tex(open.tex),
            brace_tex(close.tex)
        )))
    }

    /// One matrix cell: an expression up to `,` or the row's closing bracket.
    fn matrix_cell(&mut self) -> Result<String> {
        let mut out = String::new();
        loop {
            self.skip_ws();
            let rest = &self.src[self.pos..];
            if rest.is_empty() || rest.starts_with(',') || self.peek_kind() == Some(RightBracket) {
                return Ok(out);
            }
            let t = self.intermediate()?;
            if !out.is_empty() && needs_space(&out, &t) {
                out.push(' ');
            }
            out.push_str(&t);
        }
    }

    /// `|x|`, `||v||`: a bar opens if something follows it and closes at the
    /// next bar of the same kind.
    fn bars(&mut self, sym: &'static Sym) -> Result<String> {
        let rest = &self.src[self.pos..];
        let close = sym.input;
        match rest.find(close) {
            Some(end) => {
                let inner = &self.src[self.pos..self.pos + end];
                let mut sub = P {
                    src: inner,
                    pos: 0,
                    depth: self.depth,
                };
                let mut tex = String::new();
                while sub.skip_ws() < inner.len() {
                    let before = sub.pos;
                    tex.push_str(&sub.expression()?);
                    if sub.pos == before {
                        let (sym, len) = sub.lookup().ok_or_else(|| Error::parse(self.pos, "unexpected input"))?;
                        sub.pos += len;
                        tex.push_str(&brace_tex(sym.tex));
                    }
                }
                self.pos += end + close.len();
                Ok(format!(r"\left{}{tex}\right{}", sym.tex, sym.tex))
            }
            None => Ok(format!("{} ", sym.tex)),
        }
    }
}

fn needs_space(prev: &str, next: &str) -> bool {
    prev.ends_with(|c: char| c.is_ascii_alphabetic()) && next.starts_with(|c: char| c.is_ascii_alphabetic())
        || prev.ends_with(|c: char| c.is_ascii_digit()) && next.starts_with(|c: char| c.is_ascii_digit())
}

fn escape_text(t: &str) -> String {
    let mut out = String::new();
    for c in t.chars() {
        match c {
            '{' | '}' | '%' | '&' | '#' | '$' | '_' => {
                out.push('\\');
                out.push(c);
            }
            '\\' => out.push_str(r"\textbackslash "),
            '^' => out.push_str(r"\textasciicircum "),
            '~' => out.push_str(r"\textasciitilde "),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::to_tex;

    /// Compares what the translation parses to with what the expected TeX
    /// parses to, so spacing in the generated TeX does not matter.
    fn same(am: &str, expected: &str) {
        let tex = to_tex(am).unwrap();
        let got = crate::parse(&tex).unwrap_or_else(|e| panic!("{am} -> {tex}: {e}"));
        assert_eq!(got, crate::parse(expected).unwrap(), "{am} -> {tex}");
    }

    #[test]
    fn the_examples_from_asciimath_org() {
        same(
            "sum_(i=1)^n i^3=((n(n+1))/2)^2",
            r"\sum _{i=1}^{n} i^{3}=\left(\frac{n\left(n+1\right)}{2}\right)^{2}",
        );
        same("x/y", r"\frac{x}{y}");
        same("(a+b)/2", r"\frac{a+b}{2}");
        same("sqrt x", r"\sqrt{x}");
        same("root(3)(x)", r"\sqrt[3]{x}");
        same("int_0^1 f(x)dx", r"\int _{0}^{1} f \left(x\right)dx");
        same("[[a,b],[c,d]]", r"\left[\begin{array}{cc} a & b \\ c & d \end{array}\right]");
        same("|x|", r"\left|x\right|");
        same("abs(x)", r"\left|x\right|");
        same(r#""speed" = d/t"#, r"\text{speed} = \frac{d}{t}");
        same("a != b", r"a\ne b");
        same("x in RR", r"x\in \mathbb{R} ");
        same("lim_(x->oo) 1/x = 0", r"\lim _{x\to \infty } \frac{1}{x} = 0");
        same("hat x vec v bar y", r"\hat{x}\vec{v}\overline{y}");
        same("color(red)(x)", r"\textcolor{red}{x}");
        same("sin^2 x + cos^2 x = 1", r"\sin ^{2} x + \cos ^{2} x = 1");
    }

    #[test]
    fn hostile_input_is_an_error_not_a_crash() {
        assert!(to_tex(&"(".repeat(10_000)).is_err());
        // Whitespace outside ASCII (found by the mutation fuzzer).
        assert!(to_tex("a\u{a0}b\u{3000}c").is_ok());
        for s in ["\"", "text(", "((", "[[a,b],[c", "|", "/", "^", "_", "frac", "root(", "color("] {
            let _ = to_tex(s).map(|t| crate::parse(&t));
        }
    }
}
