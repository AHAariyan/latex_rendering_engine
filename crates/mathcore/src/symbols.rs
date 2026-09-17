//! Symbol tables: control sequence -> code point + atom class, character
//! classes for plain characters, and the mapping of letters onto the Unicode
//! Mathematical Alphanumeric Symbols block for `\mathbf` and friends.

use crate::ast::{AtomType, Variant};
use AtomType::*;

/// Symbols reachable by a control sequence. Sorted lookups are not needed;
/// the table is small enough for a linear scan and is only hit at parse time.
pub static SYMBOLS: &[(&str, char, AtomType)] = &[
    // Lowercase Greek (italic by default, handled by the variant mapper).
    ("alpha", 'α', Ord),
    ("beta", 'β', Ord),
    ("gamma", 'γ', Ord),
    ("delta", 'δ', Ord),
    ("epsilon", 'ϵ', Ord),
    ("varepsilon", 'ε', Ord),
    ("zeta", 'ζ', Ord),
    ("eta", 'η', Ord),
    ("theta", 'θ', Ord),
    ("vartheta", 'ϑ', Ord),
    ("iota", 'ι', Ord),
    ("kappa", 'κ', Ord),
    ("varkappa", 'ϰ', Ord),
    ("lambda", 'λ', Ord),
    ("mu", 'μ', Ord),
    ("nu", 'ν', Ord),
    ("xi", 'ξ', Ord),
    ("omicron", 'ο', Ord),
    ("pi", 'π', Ord),
    ("varpi", 'ϖ', Ord),
    ("rho", 'ρ', Ord),
    ("varrho", 'ϱ', Ord),
    ("sigma", 'σ', Ord),
    ("varsigma", 'ς', Ord),
    ("tau", 'τ', Ord),
    ("upsilon", 'υ', Ord),
    ("phi", 'ϕ', Ord),
    ("varphi", 'φ', Ord),
    ("chi", 'χ', Ord),
    ("psi", 'ψ', Ord),
    ("omega", 'ω', Ord),
    // Uppercase Greek (upright by default).
    ("Gamma", 'Γ', Ord),
    ("Delta", 'Δ', Ord),
    ("Theta", 'Θ', Ord),
    ("Lambda", 'Λ', Ord),
    ("Xi", 'Ξ', Ord),
    ("Pi", 'Π', Ord),
    ("Sigma", 'Σ', Ord),
    ("Upsilon", 'Υ', Ord),
    ("Phi", 'Φ', Ord),
    ("Psi", 'Ψ', Ord),
    ("Omega", 'Ω', Ord),
    // Ordinary symbols.
    ("infty", '∞', Ord),
    ("partial", '∂', Ord),
    ("nabla", '∇', Ord),
    ("hbar", 'ℏ', Ord),
    ("ell", 'ℓ', Ord),
    ("Re", 'ℜ', Ord),
    ("Im", 'ℑ', Ord),
    ("aleph", 'ℵ', Ord),
    ("beth", 'ℶ', Ord),
    ("wp", '℘', Ord),
    ("emptyset", '∅', Ord),
    ("varnothing", '∅', Ord),
    ("forall", '∀', Ord),
    ("exists", '∃', Ord),
    ("nexists", '∄', Ord),
    ("neg", '¬', Ord),
    ("lnot", '¬', Ord),
    ("top", '⊤', Ord),
    ("bot", '⊥', Ord),
    ("angle", '∠', Ord),
    ("measuredangle", '∡', Ord),
    ("triangle", '△', Ord),
    ("prime", '′', Ord),
    ("backslash", '\\', Ord),
    ("surd", '√', Ord),
    ("flat", '♭', Ord),
    ("natural", '♮', Ord),
    ("sharp", '♯', Ord),
    ("clubsuit", '♣', Ord),
    ("diamondsuit", '♢', Ord),
    ("heartsuit", '♡', Ord),
    ("spadesuit", '♠', Ord),
    ("Box", '□', Ord),
    ("square", '□', Ord),
    ("blacksquare", '■', Ord),
    ("Diamond", '◇', Ord),
    ("lozenge", '◊', Ord),
    ("bigstar", '★', Ord),
    ("checkmark", '✓', Ord),
    ("cdots", '⋯', Inner),
    ("ldots", '…', Inner),
    ("dots", '…', Inner),
    ("vdots", '⋮', Ord),
    ("ddots", '⋱', Inner),
    ("iddots", '⋰', Inner),
    ("mathellipsis", '…', Inner),
    ("hash", '#', Ord),
    ("dollar", '$', Ord),
    ("%", '%', Ord),
    ("&", '&', Ord),
    ("#", '#', Ord),
    ("$", '$', Ord),
    ("_", '_', Ord),
    ("imath", 'ı', Ord),
    ("S", '§', Ord),
    ("P", '¶', Ord),
    ("pounds", '£', Ord),
    ("copyright", '©', Ord),
    ("dag", '†', Ord),
    ("ddag", '‡', Ord),
    ("dotsb", '⋯', Inner),
    ("dotsc", '…', Inner),
    ("dotsi", '⋯', Inner),
    ("dotsm", '⋯', Inner),
    ("dotso", '…', Inner),
    ("ldotp", '.', Punct),
    ("cdotp", '·', Punct),
    ("textdollar", '$', Ord),
    ("textbackslash", '\\', Ord),
    ("mathdollar", '$', Ord),
    ("mathsterling", '£', Ord),
    ("lq", '‘', Ord),
    ("rq", '’', Ord),
    ("textquoteleft", '‘', Ord),
    ("textquoteright", '’', Ord),
    ("blacktriangle", '▴', Ord),
    ("blacktriangledown", '▾', Ord),
    ("blacklozenge", '⧫', Ord),
    ("circledS", 'Ⓢ', Ord),
    ("hslash", 'ℏ', Ord),
    ("complement", '∁', Ord),
    ("Finv", 'Ⅎ', Ord),
    ("Game", '⅁', Ord),
    ("diagup", '╱', Ord),
    ("diagdown", '╲', Ord),
    ("backprime", '‵', Ord),
    ("gimel", 'ℷ', Ord),
    ("daleth", 'ℸ', Ord),
    ("jmath", 'ȷ', Ord),
    ("degree", '°', Ord),
    ("mho", '℧', Ord),
    ("eth", 'ð', Ord),
    // Binary operators.
    ("pm", '±', Bin),
    ("mp", '∓', Bin),
    ("times", '×', Bin),
    ("div", '÷', Bin),
    ("cdot", '⋅', Bin),
    ("ast", '∗', Bin),
    ("star", '⋆', Bin),
    ("circ", '∘', Bin),
    ("bullet", '∙', Bin),
    ("cap", '∩', Bin),
    ("cup", '∪', Bin),
    ("uplus", '⊎', Bin),
    ("sqcap", '⊓', Bin),
    ("sqcup", '⊔', Bin),
    ("vee", '∨', Bin),
    ("wedge", '∧', Bin),
    ("lor", '∨', Bin),
    ("land", '∧', Bin),
    ("setminus", '∖', Bin),
    ("smallsetminus", '∖', Bin),
    ("oplus", '⊕', Bin),
    ("ominus", '⊖', Bin),
    ("otimes", '⊗', Bin),
    ("oslash", '⊘', Bin),
    ("odot", '⊙', Bin),
    ("wr", '≀', Bin),
    ("amalg", '⨿', Bin),
    ("dagger", '†', Bin),
    ("ddagger", '‡', Bin),
    ("diamond", '⋄', Bin),
    ("bigtriangleup", '△', Bin),
    ("bigtriangledown", '▽', Bin),
    ("triangleleft", '◁', Bin),
    ("triangleright", '▷', Bin),
    ("lhd", '⊲', Bin),
    ("rhd", '⊳', Bin),
    ("unlhd", '⊴', Bin),
    ("unrhd", '⊵', Bin),
    ("boxplus", '⊞', Bin),
    ("boxminus", '⊟', Bin),
    ("boxtimes", '⊠', Bin),
    ("boxdot", '⊡', Bin),
    ("circledcirc", '⊚', Bin),
    ("circledast", '⊛', Bin),
    ("circleddash", '⊝', Bin),
    ("intercal", '⊺', Bin),
    ("ltimes", '⋉', Bin),
    ("rtimes", '⋊', Bin),
    ("dotplus", '∔', Bin),
    ("centerdot", '·', Bin),
    // Relations.
    ("ne", '≠', Rel),
    ("neq", '≠', Rel),
    ("le", '≤', Rel),
    ("leq", '≤', Rel),
    ("ge", '≥', Rel),
    ("geq", '≥', Rel),
    ("leqslant", '⩽', Rel),
    ("geqslant", '⩾', Rel),
    ("leqq", '≦', Rel),
    ("geqq", '≧', Rel),
    ("lneq", '⪇', Rel),
    ("gneq", '⪈', Rel),
    ("nless", '≮', Rel),
    ("ngtr", '≯', Rel),
    ("nleq", '≰', Rel),
    ("ngeq", '≱', Rel),
    ("equiv", '≡', Rel),
    ("approx", '≈', Rel),
    ("sim", '∼', Rel),
    ("simeq", '≃', Rel),
    ("cong", '≅', Rel),
    ("propto", '∝', Rel),
    ("doteq", '≐', Rel),
    ("asymp", '≍', Rel),
    ("nsim", '≁', Rel),
    ("ncong", '≇', Rel),
    ("napprox", '≉', Rel),
    ("approxeq", '≊', Rel),
    ("subset", '⊂', Rel),
    ("supset", '⊃', Rel),
    ("subseteq", '⊆', Rel),
    ("supseteq", '⊇', Rel),
    ("subsetneq", '⊊', Rel),
    ("supsetneq", '⊋', Rel),
    ("nsubseteq", '⊈', Rel),
    ("nsupseteq", '⊉', Rel),
    ("sqsubset", '⊏', Rel),
    ("sqsupset", '⊐', Rel),
    ("sqsubseteq", '⊑', Rel),
    ("sqsupseteq", '⊒', Rel),
    ("in", '∈', Rel),
    ("notin", '∉', Rel),
    ("ni", '∋', Rel),
    ("owns", '∋', Rel),
    ("mid", '∣', Rel),
    ("nmid", '∤', Rel),
    ("parallel", '∥', Rel),
    ("nparallel", '∦', Rel),
    ("perp", '⊥', Rel),
    ("ll", '≪', Rel),
    ("gg", '≫', Rel),
    ("lll", '⋘', Rel),
    ("ggg", '⋙', Rel),
    ("prec", '≺', Rel),
    ("succ", '≻', Rel),
    ("preceq", '⪯', Rel),
    ("succeq", '⪰', Rel),
    ("models", '⊨', Rel),
    ("vdash", '⊢', Rel),
    ("dashv", '⊣', Rel),
    ("Vdash", '⊩', Rel),
    ("vDash", '⊨', Rel),
    ("bowtie", '⋈', Rel),
    ("Join", '⋈', Rel),
    ("smile", '⌣', Rel),
    ("frown", '⌢', Rel),
    ("between", '≬', Rel),
    ("pitchfork", '⋔', Rel),
    ("therefore", '∴', Rel),
    ("because", '∵', Rel),
    ("colon", ':', Punct),
    ("ratio", '∶', Rel),
    ("triangleq", '≜', Rel),
    ("circeq", '≗', Rel),
    ("eqcirc", '≖', Rel),
    ("bumpeq", '≏', Rel),
    ("Bumpeq", '≎', Rel),
    ("doteqdot", '≑', Rel),
    ("fallingdotseq", '≒', Rel),
    ("risingdotseq", '≓', Rel),
    ("backsim", '∽', Rel),
    ("backsimeq", '⋍', Rel),
    ("lesssim", '≲', Rel),
    ("gtrsim", '≳', Rel),
    ("lessgtr", '≶', Rel),
    ("gtrless", '≷', Rel),
    ("lesseqgtr", '⋚', Rel),
    ("gtreqless", '⋛', Rel),
    ("precsim", '≾', Rel),
    ("succsim", '≿', Rel),
    ("vartriangleleft", '⊲', Rel),
    ("vartriangleright", '⊳', Rel),
    ("trianglelefteq", '⊴', Rel),
    ("trianglerighteq", '⊵', Rel),
    // Arrows.
    ("to", '→', Rel),
    ("rightarrow", '→', Rel),
    ("leftarrow", '←', Rel),
    ("gets", '←', Rel),
    ("leftrightarrow", '↔', Rel),
    ("Rightarrow", '⇒', Rel),
    ("Leftarrow", '⇐', Rel),
    ("Leftrightarrow", '⇔', Rel),
    ("mapsto", '↦', Rel),
    ("longmapsto", '⟼', Rel),
    ("longrightarrow", '⟶', Rel),
    ("longleftarrow", '⟵', Rel),
    ("longleftrightarrow", '⟷', Rel),
    ("Longrightarrow", '⟹', Rel),
    ("Longleftarrow", '⟸', Rel),
    ("Longleftrightarrow", '⟺', Rel),
    ("implies", '⟹', Rel),
    ("impliedby", '⟸', Rel),
    ("iff", '⟺', Rel),
    ("uparrow", '↑', Rel),
    ("downarrow", '↓', Rel),
    ("updownarrow", '↕', Rel),
    ("Uparrow", '⇑', Rel),
    ("Downarrow", '⇓', Rel),
    ("Updownarrow", '⇕', Rel),
    ("nearrow", '↗', Rel),
    ("searrow", '↘', Rel),
    ("swarrow", '↙', Rel),
    ("nwarrow", '↖', Rel),
    ("hookleftarrow", '↩', Rel),
    ("hookrightarrow", '↪', Rel),
    ("leftharpoonup", '↼', Rel),
    ("leftharpoondown", '↽', Rel),
    ("rightharpoonup", '⇀', Rel),
    ("rightharpoondown", '⇁', Rel),
    ("rightleftharpoons", '⇌', Rel),
    ("leftrightharpoons", '⇋', Rel),
    ("twoheadrightarrow", '↠', Rel),
    ("twoheadleftarrow", '↞', Rel),
    ("rightarrowtail", '↣', Rel),
    ("leftarrowtail", '↢', Rel),
    ("nrightarrow", '↛', Rel),
    ("nleftarrow", '↚', Rel),
    ("nRightarrow", '⇏', Rel),
    ("nLeftarrow", '⇍', Rel),
    ("leadsto", '⇝', Rel),
    ("rightsquigarrow", '⇝', Rel),
    ("circlearrowleft", '↺', Rel),
    ("circlearrowright", '↻', Rel),
    ("curvearrowleft", '↶', Rel),
    ("curvearrowright", '↷', Rel),
    // Delimiters as plain symbols.
    ("langle", '⟨', Open),
    ("rangle", '⟩', Close),
    ("lfloor", '⌊', Open),
    ("rfloor", '⌋', Close),
    ("lceil", '⌈', Open),
    ("rceil", '⌉', Close),
    ("lbrace", '{', Open),
    ("rbrace", '}', Close),
    ("{", '{', Open),
    ("}", '}', Close),
    ("lbrack", '[', Open),
    ("rbrack", ']', Close),
    ("vert", '|', Ord),
    ("|", '‖', Ord),
    ("Vert", '‖', Ord),
    ("lvert", '|', Open),
    ("rvert", '|', Close),
    ("lVert", '‖', Open),
    ("rVert", '‖', Close),
    ("llbracket", '⟦', Open),
    ("rrbracket", '⟧', Close),
    ("lgroup", '⟮', Open),
    ("rgroup", '⟯', Close),
    ("lmoustache", '⎰', Open),
    ("rmoustache", '⎱', Close),
];

/// Large operators: `\sum` style symbols whose glyph grows in display style.
/// The bool is `true` when limits are placed above and below by default.
pub static BIG_OPS: &[(&str, char, bool)] = &[
    ("sum", '∑', true),
    ("prod", '∏', true),
    ("coprod", '∐', true),
    ("int", '∫', false),
    ("iint", '∬', false),
    ("iiint", '∭', false),
    ("oint", '∮', false),
    ("oiint", '∯', false),
    ("oiiint", '∰', false),
    ("intop", '∫', true),
    ("bigcup", '⋃', true),
    ("bigcap", '⋂', true),
    ("bigvee", '⋁', true),
    ("bigwedge", '⋀', true),
    ("bigoplus", '⨁', true),
    ("bigotimes", '⨂', true),
    ("bigodot", '⨀', true),
    ("biguplus", '⨄', true),
    ("bigsqcup", '⨆', true),
];

/// Function names typeset upright. The bool is `true` when scripts become
/// limits in display style (`\lim_{x\to 0}`).
pub static FN_NAMES: &[(&str, bool)] = &[
    ("sin", false),
    ("cos", false),
    ("tan", false),
    ("cot", false),
    ("sec", false),
    ("csc", false),
    ("arcsin", false),
    ("arccos", false),
    ("arctan", false),
    ("sinh", false),
    ("cosh", false),
    ("tanh", false),
    ("coth", false),
    ("exp", false),
    ("log", false),
    ("ln", false),
    ("lg", false),
    ("arg", false),
    ("deg", false),
    ("dim", false),
    ("hom", false),
    ("ker", false),
    ("lim", true),
    ("liminf", true),
    ("limsup", true),
    ("max", true),
    ("min", true),
    ("sup", true),
    ("inf", true),
    ("det", true),
    ("gcd", true),
    ("Pr", true),
];

/// Non-stretchy accents and their combining code points.
pub static ACCENTS: &[(&str, char, bool)] = &[
    ("hat", '\u{0302}', false),
    ("widehat", '\u{0302}', true),
    ("tilde", '\u{0303}', false),
    ("widetilde", '\u{0303}', true),
    ("bar", '\u{0304}', false),
    ("vec", '\u{20D7}', false),
    ("dot", '\u{0307}', false),
    ("ddot", '\u{0308}', false),
    ("dddot", '\u{20DB}', false),
    ("acute", '\u{0301}', false),
    ("grave", '\u{0300}', false),
    ("breve", '\u{0306}', false),
    ("check", '\u{030C}', false),
    ("mathring", '\u{030A}', false),
    ("overrightarrow", '\u{20D7}', true),
    ("overleftarrow", '\u{20D6}', true),
];

/// Spaces in mu (1/18 em).
pub static SPACES: &[(&str, f32)] = &[
    (",", 3.0),
    (":", 4.0),
    (";", 5.0),
    ("!", -3.0),
    (" ", 6.0),
    ("quad", 18.0),
    ("qquad", 36.0),
    ("thinspace", 3.0),
    ("medspace", 4.0),
    ("thickspace", 5.0),
    ("negthinspace", -3.0),
    ("enspace", 9.0),
];

/// Large operator typed directly as a Unicode character, e.g. `∑`.
pub fn char_big_op(c: char) -> Option<bool> {
    BIG_OPS.iter().find(|(_, ch, _)| *ch == c).map(|(_, _, limits)| *limits)
}

/// Named colors accepted by `\color` and `\textcolor` (xcolor's base set plus
/// the common web names), and `#rrggbb` / `#rgb`.
pub fn parse_color(name: &str) -> Option<[u8; 3]> {
    let name = name.trim();
    if let Some(hex) = name.strip_prefix('#') {
        let v = u32::from_str_radix(hex, 16).ok()?;
        return match hex.len() {
            6 => Some([(v >> 16) as u8, (v >> 8) as u8, v as u8]),
            3 => Some([((v >> 8) & 0xF) as u8 * 17, ((v >> 4) & 0xF) as u8 * 17, (v & 0xF) as u8 * 17]),
            _ => None,
        };
    }
    Some(match name.to_ascii_lowercase().as_str() {
        "black" => [0, 0, 0],
        "white" => [255, 255, 255],
        "red" => [255, 0, 0],
        "green" => [0, 128, 0],
        "lime" => [0, 255, 0],
        "blue" => [0, 0, 255],
        "cyan" => [0, 255, 255],
        "magenta" => [255, 0, 255],
        "yellow" => [255, 255, 0],
        "orange" => [255, 165, 0],
        "purple" => [128, 0, 128],
        "violet" => [128, 0, 255],
        "brown" => [150, 75, 0],
        "pink" => [255, 192, 203],
        "teal" => [0, 128, 128],
        "olive" => [128, 128, 0],
        "gray" | "grey" => [128, 128, 128],
        "darkgray" | "darkgrey" => [64, 64, 64],
        "lightgray" | "lightgrey" => [192, 192, 192],
        "navy" => [0, 0, 128],
        "maroon" => [128, 0, 0],
        _ => return None,
    })
}

pub fn lookup_symbol(name: &str) -> Option<(char, AtomType)> {
    SYMBOLS.iter().find(|(n, _, _)| *n == name).map(|(_, c, a)| (*c, *a))
}

/// Atom class of a character typed directly, following plain TeX's mathcodes.
/// Unicode symbols that have a control-sequence equivalent take its class.
pub fn char_atom(c: char) -> AtomType {
    if !c.is_ascii() {
        if let Some((_, _, atom)) = SYMBOLS.iter().find(|(_, ch, _)| *ch == c) {
            return *atom;
        }
    }
    match c {
        '+' | '-' | '*' | '±' | '×' | '÷' | '⋅' | '∘' => Bin,
        '=' | '<' | '>' | ':' | '≤' | '≥' | '≠' | '≈' | '∈' | '→' | '←' | '↔' | '⇒' | '⇐' => Rel,
        '(' | '[' | '{' | '⟨' | '⌊' | '⌈' => Open,
        ')' | ']' | '}' | '⟩' | '⌋' | '⌉' | '!' | '?' => Close,
        ',' | ';' => Punct,
        _ => Ord,
    }
}

/// Characters that may follow `\left`, `\right` and `\big`.
pub fn delimiter(name_or_char: &str) -> Option<char> {
    Some(match name_or_char {
        "(" => '(',
        ")" => ')',
        "[" => '[',
        "]" => ']',
        "{" | "lbrace" => '{',
        "}" | "rbrace" => '}',
        "|" | "vert" | "lvert" | "rvert" => '|',
        "\\|" | "Vert" | "lVert" | "rVert" => '‖',
        "langle" => '⟨',
        "rangle" => '⟩',
        "lfloor" => '⌊',
        "rfloor" => '⌋',
        "lceil" => '⌈',
        "rceil" => '⌉',
        "/" => '/',
        "backslash" => '\\',
        "uparrow" => '↑',
        "downarrow" => '↓',
        "updownarrow" => '↕',
        "Uparrow" => '⇑',
        "Downarrow" => '⇓',
        "Updownarrow" => '⇕',
        "llbracket" => '⟦',
        "rrbracket" => '⟧',
        "lgroup" => '⟮',
        "rgroup" => '⟯',
        "lmoustache" => '⎰',
        "rmoustache" => '⎱',
        "<" => '⟨',
        ">" => '⟩',
        _ => return None,
    })
}

/// Maps a base character to the Unicode Mathematical Alphanumeric Symbols
/// code point for the requested variant. Characters outside the Latin, Greek
/// and digit ranges are returned unchanged.
pub fn styled_char(ch: char, variant: Variant) -> char {
    // Resolve TeX's default: italic Latin and lowercase Greek, upright otherwise.
    let variant = match variant {
        Variant::Normal => {
            if ch.is_ascii_alphabetic() || ('α'..='ω').contains(&ch) || matches!(ch, 'ϵ' | 'ϑ' | 'ϰ' | 'ϖ' | 'ϱ' | 'ϕ' | '∂') {
                Variant::Italic
            } else {
                return ch;
            }
        }
        v => v,
    };
    if let Some(c) = styled_latin(ch, variant) {
        return c;
    }
    if let Some(c) = styled_greek(ch, variant) {
        return c;
    }
    if let Some(c) = styled_digit(ch, variant) {
        return c;
    }
    ch
}

fn offset_char(base: u32, idx: u32) -> char {
    char::from_u32(base + idx).unwrap_or('\u{FFFD}')
}

fn styled_latin(ch: char, v: Variant) -> Option<char> {
    let (upper, idx) = if ch.is_ascii_uppercase() {
        (true, ch as u32 - 'A' as u32)
    } else if ch.is_ascii_lowercase() {
        (false, ch as u32 - 'a' as u32)
    } else {
        return None;
    };
    // Reserved code points in the alphanumeric block that live elsewhere in Unicode.
    let holes: &[(Variant, bool, u32, char)] = &[
        (Variant::Italic, false, 7, 'ℎ'),
        (Variant::Script, true, 1, 'ℬ'),
        (Variant::Script, true, 4, 'ℰ'),
        (Variant::Script, true, 5, 'ℱ'),
        (Variant::Script, true, 7, 'ℋ'),
        (Variant::Script, true, 8, 'ℐ'),
        (Variant::Script, true, 11, 'ℒ'),
        (Variant::Script, true, 12, 'ℳ'),
        (Variant::Script, true, 17, 'ℛ'),
        (Variant::Script, false, 4, 'ℯ'),
        (Variant::Script, false, 6, 'ℊ'),
        (Variant::Script, false, 14, 'ℴ'),
        (Variant::Fraktur, true, 2, 'ℭ'),
        (Variant::Fraktur, true, 7, 'ℌ'),
        (Variant::Fraktur, true, 8, 'ℑ'),
        (Variant::Fraktur, true, 17, 'ℜ'),
        (Variant::Fraktur, true, 25, 'ℨ'),
        (Variant::DoubleStruck, true, 2, 'ℂ'),
        (Variant::DoubleStruck, true, 7, 'ℍ'),
        (Variant::DoubleStruck, true, 13, 'ℕ'),
        (Variant::DoubleStruck, true, 15, 'ℙ'),
        (Variant::DoubleStruck, true, 16, 'ℚ'),
        (Variant::DoubleStruck, true, 17, 'ℝ'),
        (Variant::DoubleStruck, true, 25, 'ℤ'),
    ];
    if let Some((_, _, _, c)) = holes.iter().find(|(hv, hu, hi, _)| *hv == v && *hu == upper && *hi == idx) {
        return Some(*c);
    }
    let base = match (v, upper) {
        (Variant::Roman, _) => return Some(ch),
        (Variant::Bold, true) => 0x1D400,
        (Variant::Bold, false) => 0x1D41A,
        (Variant::Italic, true) => 0x1D434,
        (Variant::Italic, false) => 0x1D44E,
        (Variant::BoldItalic, true) => 0x1D468,
        (Variant::BoldItalic, false) => 0x1D482,
        (Variant::Script, true) => 0x1D49C,
        (Variant::Script, false) => 0x1D4B6,
        (Variant::Fraktur, true) => 0x1D504,
        (Variant::Fraktur, false) => 0x1D51E,
        (Variant::DoubleStruck, true) => 0x1D538,
        (Variant::DoubleStruck, false) => 0x1D552,
        (Variant::SansSerif, true) => 0x1D5A0,
        (Variant::SansSerif, false) => 0x1D5BA,
        (Variant::Monospace, true) => 0x1D670,
        (Variant::Monospace, false) => 0x1D68A,
        (Variant::Normal, _) => unreachable!(),
    };
    Some(offset_char(base, idx))
}

fn styled_greek(ch: char, v: Variant) -> Option<char> {
    // Index into the 58-glyph Greek run used by each math alphabet.
    let idx = match ch {
        // U+03A2 is unassigned in Greek, and the math alphabets put ϴ in that slot.
        'Α'..='Ω' => ch as u32 - 'Α' as u32,
        'ϴ' => 17,
        '∇' => 25,
        'α'..='ω' => 26 + (ch as u32 - 'α' as u32),
        '∂' => 51,
        'ϵ' => 52,
        'ϑ' => 53,
        'ϰ' => 54,
        'ϕ' => 55,
        'ϱ' => 56,
        'ϖ' => 57,
        _ => return None,
    };
    let base = match v {
        Variant::Roman | Variant::SansSerif | Variant::Monospace | Variant::Script | Variant::Fraktur | Variant::DoubleStruck => {
            return Some(ch)
        }
        Variant::Bold => 0x1D6A8,
        Variant::Italic => 0x1D6E2,
        Variant::BoldItalic => 0x1D71C,
        Variant::Normal => unreachable!(),
    };
    Some(offset_char(base, idx))
}

fn styled_digit(ch: char, v: Variant) -> Option<char> {
    if !ch.is_ascii_digit() {
        return None;
    }
    let idx = ch as u32 - '0' as u32;
    let base = match v {
        Variant::Bold | Variant::BoldItalic => 0x1D7CE,
        Variant::DoubleStruck => 0x1D7D8,
        Variant::SansSerif => 0x1D7E2,
        Variant::Monospace => 0x1D7F6,
        _ => return Some(ch),
    };
    Some(offset_char(base, idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_italic_mapping() {
        assert_eq!(styled_char('a', Variant::Normal), '𝑎');
        assert_eq!(styled_char('h', Variant::Normal), 'ℎ');
        assert_eq!(styled_char('1', Variant::Normal), '1');
        assert_eq!(styled_char('Γ', Variant::Normal), 'Γ');
        assert_eq!(styled_char('α', Variant::Normal), '𝛼');
        assert_eq!(styled_char('ω', Variant::Normal), '𝜔');
        assert_eq!(styled_char('R', Variant::DoubleStruck), 'ℝ');
        assert_eq!(styled_char('A', Variant::DoubleStruck), '𝔸');
        assert_eq!(styled_char('x', Variant::Bold), '𝐱');
        assert_eq!(styled_char('x', Variant::Roman), 'x');
        assert_eq!(styled_char('B', Variant::Script), 'ℬ');
        assert_eq!(styled_char('Ω', Variant::Bold), '𝛀');
    }
}
