//! The `physics` package: `\dv`, `\pdv`, `\abs`, `\norm`, `\qty`, Dirac
//! notation, vector operators and matrices.
//!
//! Like mhchem, each command is translated to ordinary TeX that the main
//! parser then reads. Commands whose names other packages also define keep
//! their usual meaning unless their arguments show the physics form:
//! `\div` is ÷ unless an argument follows (`\div{\vb{E}}`), `\braket` takes
//! physics' two arguments only when two are given, and `\qty{1}{m}` with two
//! groups is siunitx's quantity.

use crate::error::Result;
use crate::lexer::Lexer;

/// Translates a physics command whose name has just been read, or returns
/// `None` (consuming nothing but a `*`) when `name` is not one, or is one in
/// a form the main parser handles itself.
pub(super) fn translate(name: &str, lx: &mut Lexer<'_>) -> Result<Option<String>> {
    let next = lx.peek_raw_skipping_space();
    Ok(Some(match name {
        "qty" => {
            let star = lx.eat('*');
            if next_is(lx, '{') {
                let a = lx.raw_group()?;
                if next_is(lx, '{') {
                    // siunitx's \qty{number}{unit}.
                    let unit = lx.raw_group()?;
                    return Ok(Some(super::siunitx::quantity(a, unit)));
                }
                sized(star, r"\{", a, r"\}")
            } else {
                match delimited(lx)? {
                    Some((l, inner, r)) => sized(star, l, inner, r),
                    None => return Ok(None),
                }
            }
        }
        "pqty" | "bqty" | "vqty" | "Bqty" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            let (l, r) = match name {
                "pqty" => ("(", ")"),
                "bqty" => ("[", "]"),
                "vqty" => ("|", "|"),
                _ => (r"\{", r"\}"),
            };
            sized(star, l, a, r)
        }
        "abs" | "norm" => {
            let star = lx.eat('*');
            let a = arg_or_delimited(lx)?;
            let (l, r) = if name == "abs" {
                (r"\lvert", r"\rvert")
            } else {
                (r"\lVert", r"\rVert")
            };
            sized(star, l, &a, r)
        }
        "eval" | "evaluated" => {
            let star = lx.eat('*');
            let (l, a) = match lx.peek_raw_skipping_space() {
                Some('(') => ("(", lx.raw_balanced('(', '|')?),
                Some('[') => ("[", lx.raw_balanced('[', '|')?),
                _ => (".", lx.raw_group()?),
            };
            // physics makes the bar at least as tall as an integral, starred or not.
            let _ = star;
            format!(r"\left{l} {a} \vphantom{{\int}}\right\rvert ")
        }
        "order" => {
            let star = lx.eat('*');
            let a = arg_or_delimited(lx)?;
            format!(r"\mathcal{{O}}{}", applied(star, "(", &a, ")"))
        }
        "comm" | "commutator" | "acomm" | "anticommutator" | "pb" | "poissonbracket" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            let b = lx.raw_group()?;
            let (l, r) = if name.starts_with('c') { ("[", "]") } else { (r"\{", r"\}") };
            sized(star, l, &format!("{a},{b}"), r)
        }
        "bra" | "ket" => {
            // Starred is unsized, which is also how the main parser draws them.
            lx.eat('*');
            return Ok(None);
        }
        "braket" | "innerproduct" | "ip" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            if next_is(lx, '{') {
                let b = lx.raw_group()?;
                dirac(star, &[a, b])
            } else if name == "braket" {
                // The braket package's one-argument form: `\braket{\phi|\psi}`.
                format!(r"\mathinner{{\langle {a}\rangle}}")
            } else {
                dirac(star, &[a, a])
            }
        }
        "ketbra" | "outerproduct" | "dyad" | "op" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            let b = if next_is(lx, '{') { lx.raw_group()? } else { a };
            if star {
                format!(r"\lvert {a}\rangle\!\langle {b}\rvert")
            } else {
                format!(r"\left\lvert {a}\right\rangle\!\left\langle {b}\right\rvert")
            }
        }
        "expval" | "expectationvalue" | "ev" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            if next_is(lx, '{') {
                let psi = lx.raw_group()?;
                dirac(star, &[psi, a, psi])
            } else {
                sized(star, r"\langle", a, r"\rangle")
            }
        }
        "mel" | "matrixel" | "matrixelement" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            let b = lx.raw_group()?;
            let c = lx.raw_group()?;
            dirac(star, &[a, b, c])
        }
        "dv" | "derivative" | "pdv" | "partialderivative" | "pd" | "fdv" | "functionalderivative" | "fderivative" => {
            let d = match name {
                "dv" | "derivative" => r"\mathrm{d}",
                "fdv" | "functionalderivative" | "fderivative" => r"\delta",
                _ => r"\partial",
            };
            let inline = lx.eat('*');
            let order = lx.raw_optional()?.map(str::trim).filter(|n| !n.is_empty());
            let mut args = vec![lx.raw_group()?];
            while args.len() < 3 && next_is(lx, '{') {
                args.push(lx.raw_group()?);
            }
            let up = order.map_or(String::new(), |n| format!("^{{{n}}}"));
            let (num, den) = match args.as_slice() {
                [x] => (format!("{d}{up}"), format!("{d} {x}{up}")),
                [f, x] => (format!("{d}{up} {f}"), format!("{d} {x}{up}")),
                [f, x, y] => (format!("{d}^{{2}} {f}"), format!("{d} {x} {d} {y}")),
                _ => unreachable!(),
            };
            let applied = match lx.peek_raw_skipping_space() {
                Some('(') if args.len() == 1 => applied(false, "(", lx.raw_balanced('(', ')')?, ")"),
                _ => String::new(),
            };
            if inline {
                format!("{{{num}}}/{{{den}}}{applied}")
            } else {
                format!(r"\frac{{{num}}}{{{den}}}{applied}")
            }
        }
        "dd" | "differential" | "var" | "variation" => {
            let d = if name.starts_with('d') { r"\mathrm{d}" } else { r"\delta" };
            let order = lx.raw_optional()?.map(str::trim).filter(|n| !n.is_empty());
            let power = order.map_or(String::new(), |n| format!("^{{{n}}}"));
            // With an argument the differential is an Inner atom, spaced
            // from what it multiplies: `x \dd{x}`.
            match lx.peek_raw_skipping_space() {
                Some('{') => format!(r"\mathinner{{{d}{power} {}}}", lx.raw_group()?),
                Some('(') => format!(r"\mathinner{{{d}{power}{}}}", applied(false, "(", lx.raw_balanced('(', ')')?, ")")),
                _ => format!("{{{d}{power}}}"),
            }
        }
        "vb" | "vectorbold" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            if star {
                format!(r"\boldsymbol{{{a}}}")
            } else {
                format!(r"\mathbf{{{a}}}")
            }
        }
        "va" | "vectorarrow" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            if star {
                format!(r"\vec{{\boldsymbol{{{a}}}}}")
            } else {
                format!(r"\vec{{\mathbf{{{a}}}}}")
            }
        }
        "vu" | "vectorunit" => {
            let star = lx.eat('*');
            let a = lx.raw_group()?;
            if star {
                format!(r"\hat{{\boldsymbol{{{a}}}}}")
            } else {
                format!(r"\hat{{\mathbf{{{a}}}}}")
            }
        }
        "vdot" | "dotproduct" => r"\mathbin{\boldsymbol{\cdot}}".into(),
        "cross" | "crossproduct" | "cp" => r"\mathbin{\boldsymbol{\times}}".into(),
        "grad" | "gradient" | "curl" | "laplacian" => {
            let op = match name {
                "curl" => r"\boldsymbol{\nabla}\mathbin{\boldsymbol{\times}}",
                "laplacian" => r"\nabla^{2}",
                _ => r"\boldsymbol{\nabla}",
            };
            format!("{op}{}", operand(lx, true)?)
        }
        // Without an argument `\div` is the division sign.
        "div" | "divergence" if name == "divergence" || matches!(next, Some('{' | '(')) => {
            format!(r"\boldsymbol{{\nabla}}\mathbin{{\boldsymbol{{\cdot}}}}{}", operand(lx, false)?)
        }
        // Operators that take their argument in visible brackets:
        // `\tr{\rho}` is tr{ρ}, `\Tr(\rho)` is Tr(ρ).
        "tr" | "trace" | "Tr" | "Trace" | "rank" | "erf" | "Res" | "Residue" => {
            let op = match name {
                "trace" => "tr",
                "Trace" => "Tr",
                "Residue" => "Res",
                n => n,
            };
            let arg = match lx.peek_raw_skipping_space() {
                Some('{') => applied(false, r"\{", lx.raw_group()?, r"\}"),
                Some('(') => applied(false, "(", lx.raw_balanced('(', ')')?, ")"),
                Some('[') => applied(false, "[", lx.raw_balanced('[', ']')?, "]"),
                _ => String::new(),
            };
            format!(r"\operatorname{{{op}}}{arg}")
        }
        "pv" | "principalvalue" => r"\mathcal{P}".into(),
        "PV" => r"\operatorname{P.V.}".into(),
        "qq" => format!(r"\quad\text{{{}}}\quad", lx.raw_group()?),
        "qc" | "qcomma" => r",\quad".into(),
        "qcc" | "qcc*" => r"\quad\text{c.c.}\quad".into(),
        _ if name.len() > 1 && name.starts_with('q') && QUAD_WORDS.contains(&&name[1..]) => {
            let word = if name == "qotherwise" { "otherwise" } else { &name[1..] };
            format!(r"\quad\text{{{word}}}\quad")
        }
        "mqty" | "matrixquantity" | "pmqty" | "bmqty" | "vmqty" | "Pmqty" | "smqty" => {
            lx.eat('*');
            let (env, body) = match name {
                "pmqty" | "Pmqty" => ("pmatrix", lx.raw_group()?),
                "bmqty" => ("bmatrix", lx.raw_group()?),
                "vmqty" => ("vmatrix", lx.raw_group()?),
                "smqty" => ("smallmatrix", lx.raw_group()?),
                _ => match lx.peek_raw_skipping_space() {
                    Some('(') => ("pmatrix", lx.raw_balanced('(', ')')?),
                    Some('[') => ("bmatrix", lx.raw_balanced('[', ']')?),
                    Some('|') => ("vmatrix", lx.raw_balanced('|', '|')?),
                    _ => ("matrix", lx.raw_group()?),
                },
            };
            format!(r"\begin{{{env}}}{body}\end{{{env}}}")
        }
        _ => return Ok(None),
    }))
}

/// Words physics sets between quads: `\qand` is `\quad\text{and}\quad`.
const QUAD_WORDS: &[&str] = &[
    "and",
    "or",
    "if",
    "then",
    "else",
    "otherwise",
    "unless",
    "given",
    "using",
    "assume",
    "since",
    "let",
    "for",
    "all",
    "even",
    "odd",
    "integer",
    "as",
    "in",
];

fn next_is(lx: &mut Lexer<'_>, c: char) -> bool {
    lx.peek_raw_skipping_space() == Some(c)
}

/// `(...)`, `[...]` or `|...|` with the delimiters as TeX.
fn delimited<'a>(lx: &mut Lexer<'a>) -> Result<Option<(&'static str, &'a str, &'static str)>> {
    Ok(match lx.peek_raw_skipping_space() {
        Some('(') => Some(("(", lx.raw_balanced('(', ')')?, ")")),
        Some('[') => Some(("[", lx.raw_balanced('[', ']')?, "]")),
        Some('|') => Some((r"\lvert", lx.raw_balanced('|', '|')?, r"\rvert")),
        _ => None,
    })
}

/// A braced argument, or `(...)` taken as one: `\abs{x}`, `\order(x^2)`.
fn arg_or_delimited(lx: &mut Lexer<'_>) -> Result<String> {
    Ok(match lx.peek_raw_skipping_space() {
        Some('(') => lx.raw_balanced('(', ')')?.to_string(),
        _ => lx.raw_group()?.to_string(),
    })
}

/// What a vector operator applies to: `\grad{\phi}`, `\grad(\phi)`, or
/// nothing. The gradient's brackets are a function's (`applied`); the
/// divergence's and curl's a quantity's (`sized`).
fn operand(lx: &mut Lexer<'_>, function: bool) -> Result<String> {
    let wrap = |l, body, r| {
        if function {
            applied(false, l, body, r)
        } else {
            sized(false, l, body, r)
        }
    };
    Ok(match lx.peek_raw_skipping_space() {
        Some('{') => lx.raw_group()?.to_string(),
        Some('(') => wrap("(", lx.raw_balanced('(', ')')?, ")"),
        Some('[') => wrap("[", lx.raw_balanced('[', ']')?, "]"),
        _ => String::new(),
    })
}

/// `l body r`, growing with the body unless starred, as one ordinary atom
/// (physics' `\braces`): `\qty(x)` spaces like `x`.
fn sized(star: bool, l: &str, body: &str, r: &str) -> String {
    if star {
        format!(r"{{\mathopen{{{l}}}{{{body}}}\mathclose{{{r}}}}}")
    } else {
        format!(r"{{\left{l} {{{body}}} \right{r}}}")
    }
}

/// A function's bracketed argument (physics' `\fbraces`): no space after
/// the function, and the closing bracket spaced like `)`: `\order{h^2}` is 𝒪(h²).
fn applied(star: bool, l: &str, body: &str, r: &str) -> String {
    if star {
        format!(r"\mathopen{{}}\mathclose{{\mathopen{{{l}}}{{{body}}}\mathclose{{{r}}}}}")
    } else {
        format!(r"\mathopen{{}}\mathclose{{\left{l} {{{body}}} \right{r}}}")
    }
}

/// `⟨a|b⟩`, `⟨a|b|c⟩`, the bars growing with the brackets.
fn dirac(star: bool, parts: &[&str]) -> String {
    if star {
        format!(r"\langle {}\rangle", parts.join(r"\vert "))
    } else {
        format!(r"\left\langle {} \right\rangle", parts.join(r" \middle\vert "))
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::parse;

    #[test]
    fn every_form_parses() {
        for tex in [
            r"\dv{f}{x}",
            r"\dv[2]{f}{x}",
            r"\dv{x}(x^2+1)",
            r"\dv*{f}{t}",
            r"\pdv{f}{x}{y}",
            r"\pdv[n]{\psi}{t}",
            r"\fdv{F}{g}",
            r"\abs{x}",
            r"\abs*{\frac{1}{2}}",
            r"\norm{\vb{v}}",
            r"\qty(\frac{a}{b})",
            r"\qty[x]",
            r"\qty|x|",
            r"\qty{x}",
            r"\qty{9.81}{m/s^2}",
            r"\eval{x^2}_0^1",
            r"\eval(f(x)|_{0}^{\infty}",
            r"\order{h^2}",
            r"\comm{A}{B} = \acomm{A}{B}",
            r"\bra{\psi}\ket{\phi} \ket*{0}",
            r"\braket{a}{b} \braket{\phi|\psi} \ketbra{0}{1} \dyad{0}",
            r"\expval{H} \expval{H}{\psi} \mel{n}{V}{m}",
            r"\int f(x) \dd{x} \dd[3]{r} \var{S}",
            r"\vb{E} \vb*{\sigma} \va{a} \vu{r}",
            r"\vb{a}\vdot\vb{b}\cross\vb{c}",
            r"\grad\phi \grad{\phi} \div{\vb{E}} \curl(\vb{B}) \laplacian\psi",
            r"a \div b",
            r"\tr\rho = \Tr(\rho) \rank A \erf x",
            r"x \qq{for} y \qc z \qand w",
            r"\mqty(a & b \\ c & d) \mqty[1 & 0 \\ 0 & 1] \mqty|a & b \\ c & d| \pmqty{1 \\ 2}",
        ] {
            parse(tex).unwrap_or_else(|e| panic!("{tex}: {e}"));
        }
    }

    #[test]
    fn plain_meanings_survive() {
        // ÷ when no argument follows; braket package's one-argument form.
        assert_eq!(parse(r"a \div b").unwrap(), parse("a ÷ b").unwrap());
        assert!(parse(r"\braket{\phi|\psi}").is_ok());
    }

    #[test]
    fn nesting_is_bounded() {
        let deep = r"\abs{".repeat(200) + "x" + &"}".repeat(200);
        assert!(parse(&deep).is_err());
        let wide = r"\braket{".repeat(30) + "x" + &"}".repeat(30);
        let _ = parse(&wide);
    }
}
