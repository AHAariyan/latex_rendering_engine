//! Abstract syntax tree for TeX math.
//!
//! The tree mirrors TeX's "noad" structure (TeXbook chapter 17) rather than the
//! surface syntax: every node knows which spacing class it belongs to, so the
//! layout engine never has to look back at command names.

use crate::display::Color;

/// TeX atom classes. The inter-atom spacing table is indexed by these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomType {
    Ord,
    Op,
    Bin,
    Rel,
    Open,
    Close,
    Punct,
    Inner,
}

/// Font variant for letters and digits, mapped onto the Unicode
/// Mathematical Alphanumeric Symbols block at layout time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    /// TeX default: italic Latin and lowercase Greek, upright digits and uppercase Greek.
    Normal,
    Roman,
    Bold,
    Italic,
    BoldItalic,
    Script,
    Fraktur,
    DoubleStruck,
    SansSerif,
    Monospace,
}

/// The four TeX math styles, ordered from smallest to largest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MathStyle {
    ScriptScript,
    Script,
    Text,
    Display,
}

/// Where the scripts of a large operator go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Limits {
    /// Limits in display style, scripts otherwise (TeX's default for `\sum`).
    Default,
    Limits,
    NoLimits,
}

/// Horizontal alignment of an array column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColAlign {
    Left,
    Center,
    Right,
}

/// The rule of a generalized fraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FracRule {
    /// Font's `fractionRuleThickness`.
    Default,
    /// No rule (`\binom`, `\atop`).
    None,
    /// Explicit thickness in em.
    Custom(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhantomKind {
    /// Width, height and depth, no ink.
    Full,
    /// Width only.
    Horizontal,
    /// Height and depth only.
    Vertical,
    /// Ink and width, but zero height and depth.
    Smash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelKind {
    /// Bottom-left to top-right (`\cancel`).
    Up,
    /// Top-left to bottom-right (`\bcancel`).
    Down,
    /// Both (`\xcancel`).
    Cross,
}

/// Byte range of the source that produced a node, for hit testing. Offsets are
/// into the source the parser saw, which is the caller's string unless macros
/// expanded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

/// A delimiter for `\left` / `\right`. `None` is the null delimiter `.`.
pub type Delim = Option<char>;

/// How rows of an array are spaced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowPitch {
    /// LaTeX `array`: struts of 0.84/0.36 em and a 1.2 em baseline pitch.
    Normal,
    /// amsmath `smallmatrix`: 0.6 em pitch, 0.15 em lineskip, no struts, thin-space column padding.
    SmallMatrix,
    /// amsmath `\substack`: rows stacked at 0.3 em lineskip, no struts.
    Substack,
}

/// A table of cells with optional rules and extra row spacing.
#[derive(Debug, Clone, PartialEq)]
pub struct Array {
    /// `rows[r][c]` is the cell's node list.
    pub rows: Vec<Vec<Vec<Node>>>,
    pub cols: Vec<ColAlign>,
    pub cell_style: MathStyle,
    /// Row indices before which a horizontal rule is drawn; `rows.len()` is the bottom rule.
    pub hlines: Vec<usize>,
    /// Column indices before which a vertical rule is drawn; `cols.len()` is the right rule.
    pub vlines: Vec<usize>,
    /// Extra space in em after each row (`\\[2pt]`).
    pub row_gaps: Vec<f32>,
    /// Row pitch rules.
    pub pitch: RowPitch,
    /// LaTeX `\arraystretch`: multiplies row struts and pitch (`cases` uses 1.2).
    pub stretch: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// A single character. `ch` is the base character; the variant is
    /// resolved to a concrete code point during layout.
    Symbol {
        ch: char,
        atom: AtomType,
        variant: Variant,
    },
    /// A braced group or an implicit list.
    Row(Vec<Node>),
    /// A nucleus with optional superscript and subscript.
    Scripts {
        base: Box<Node>,
        sup: Option<Box<Node>>,
        sub: Option<Box<Node>>,
    },
    /// A large operator such as `\sum` or `\int`.
    BigOp {
        ch: char,
        limits: Limits,
    },
    /// A function name such as `\sin`, typeset upright with `Op` spacing.
    FnName {
        name: String,
        limits: Limits,
    },
    /// Generalized fraction.
    /// Generalized fraction. `delims` are sized by TeX rule 15e (a fixed size
    /// per style), unlike `\left`/`\right` which follow the content.
    Frac {
        num: Box<Node>,
        den: Box<Node>,
        rule: FracRule,
        style: Option<MathStyle>,
        delims: Option<(Delim, Delim)>,
    },
    Sqrt {
        radicand: Box<Node>,
        index: Option<Box<Node>>,
    },
    LeftRight {
        left: Delim,
        body: Vec<Node>,
        right: Delim,
    },
    /// `\middle` delimiter; only valid inside `LeftRight`.
    Middle(char),
    /// Fixed-size delimiter produced by `\big`, `\Big`, `\bigg`, `\Bigg`. `size` is 1..=4.
    SizedDelim {
        ch: char,
        size: u8,
        atom: AtomType,
    },
    Accent {
        ch: char,
        base: Box<Node>,
        stretchy: bool,
    },
    Overline(Box<Node>),
    Underline(Box<Node>),
    /// Explicit style change (`\displaystyle` etc.) affecting the rest of the group.
    Style {
        style: MathStyle,
        body: Vec<Node>,
    },
    /// Verbatim text from `\text{}`; spaces are preserved.
    Text {
        text: String,
        variant: Variant,
    },
    /// Horizontal space in math units (1 mu = 1/18 em).
    Space {
        mu: f32,
    },
    Array(Box<Array>),
    Phantom {
        body: Box<Node>,
        kind: PhantomKind,
    },
    /// `\overset` / `\underset`: material stacked over or under a base that
    /// keeps its own atom class (`\overset{?}{=}` still spaces like a relation).
    OverUnder {
        base: Box<Node>,
        over: Option<Box<Node>>,
        under: Option<Box<Node>>,
    },
    /// `\color` / `\textcolor`.
    Color {
        color: Color,
        body: Vec<Node>,
    },
    /// `\boxed`.
    Boxed(Box<Node>),
    Cancel {
        body: Box<Node>,
        kind: CancelKind,
    },
    /// `\underbrace` / `\overbrace`. Scripts attached to it become limits.
    HBrace {
        base: Box<Node>,
        over: bool,
    },
    /// `\xrightarrow` and friends: a stretchy arrow with material above and below.
    XArrow {
        ch: char,
        over: Option<Box<Node>>,
        under: Option<Box<Node>>,
    },
    /// `\mathop`, `\mathrel`, ...: reclassify a group.
    Class {
        atom: AtomType,
        body: Box<Node>,
        limits: Limits,
    },
    /// Records where in the source an atom came from. The parser wraps every
    /// element of a list, so the wrappers nest with the formula's structure and
    /// a point in the drawing maps back to a range of source.
    Spanned {
        span: Span,
        body: Box<Node>,
    },
}

impl Node {
    /// The node itself with any span wrapper removed.
    pub fn bare(&self) -> &Node {
        match self {
            Node::Spanned { body, .. } => body.bare(),
            other => other,
        }
    }

    pub fn row(nodes: Vec<Node>) -> Node {
        if nodes.len() == 1 {
            nodes.into_iter().next().unwrap()
        } else {
            Node::Row(nodes)
        }
    }
}
