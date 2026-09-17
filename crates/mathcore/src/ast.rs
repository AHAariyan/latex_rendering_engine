//! Abstract syntax tree for TeX math.
//!
//! The tree mirrors TeX's "noad" structure (TeXbook chapter 17) rather than the
//! surface syntax: every node knows which spacing class it belongs to, so the
//! layout engine never has to look back at command names.

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

/// The four TeX math styles, ordered from largest to smallest.
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

/// A delimiter for `\left` / `\right`. `None` is the null delimiter `.`.
pub type Delim = Option<char>;

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
    /// Generalized fraction. `binom` is a fraction without a rule wrapped in parentheses.
    Frac {
        num: Box<Node>,
        den: Box<Node>,
        rule: bool,
        style: Option<MathStyle>,
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
    /// Table of cells: `rows[r][c]` is the cell's node list.
    Array {
        rows: Vec<Vec<Vec<Node>>>,
        cols: Vec<ColAlign>,
        cell_style: MathStyle,
    },
    /// A box with the size of its content but no ink.
    Phantom(Box<Node>),
    /// `\overset` / `\underset`: material stacked over or under a base that
    /// keeps its own atom class (`\overset{?}{=}` still spaces like a relation).
    OverUnder {
        base: Box<Node>,
        over: Option<Box<Node>>,
        under: Option<Box<Node>>,
    },
}

impl Node {
    pub fn row(nodes: Vec<Node>) -> Node {
        if nodes.len() == 1 {
            nodes.into_iter().next().unwrap()
        } else {
            Node::Row(nodes)
        }
    }
}
