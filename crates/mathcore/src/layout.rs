//! Layout engine: `Node` tree -> boxes -> `DisplayList`.
//!
//! This is an implementation of the TeXbook Appendix G rules with every
//! dimension taken from the font's OpenType MATH table instead of TeX's
//! fontdimens. Boxes use TeX conventions internally: the origin is on the
//! baseline at the left edge and y grows upward. `flatten` converts to the
//! y-down pixel space of the display list at the very end.

use crate::ast::*;
use crate::display::{Color, DisplayList, Item};
use crate::font::{KernCorner, MathFont};
use crate::macros::Macros;
use crate::symbols::styled_char;
use ttf_parser::GlyphId;

/// Breaks a formula too wide for the available space into several lines.
///
/// TeX does not break display math at all and leaves it to the author
/// (`multline`, `split`), which is no help on a phone. This follows the
/// convention those environments use by hand: break before a relation or a
/// binary operator, and start the new line with that operator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineBreak {
    /// Available width in pixels.
    pub max_width: f32,
    /// Indent of continuation lines, in em.
    pub indent: f32,
}

impl LineBreak {
    pub fn new(max_width: f32) -> Self {
        LineBreak { max_width, indent: 2.0 }
    }
}

#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Em size in pixels of the outermost formula.
    pub font_size: f32,
    /// `true` for `$$...$$` (display style), `false` for inline (text style).
    pub display_mode: bool,
    pub color: Color,
    /// Host-supplied macro definitions.
    pub macros: Macros,
    /// Break the formula to fit a width. `None` renders one line of any width.
    pub line_break: Option<LineBreak>,
    /// Caps on the work one formula may cost. Matters for untrusted input.
    pub budget: crate::Budget,
    /// Record source regions so a point in the drawing can be mapped back to a
    /// range of source. Costs one small record per atom.
    pub hit_testing: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            font_size: 32.0,
            display_mode: true,
            color: Color::BLACK,
            macros: Macros::new(),
            line_break: None,
            budget: crate::Budget::default(),
            hit_testing: false,
        }
    }
}

/// TeX style with the cramped flag (TeXbook p. 140).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Sty {
    style: MathStyle,
    cramped: bool,
}

impl Sty {
    fn is_display(self) -> bool {
        self.style == MathStyle::Display
    }
    fn sup(self) -> Sty {
        let style = match self.style {
            MathStyle::Display | MathStyle::Text => MathStyle::Script,
            _ => MathStyle::ScriptScript,
        };
        Sty {
            style,
            cramped: self.cramped,
        }
    }
    fn sub(self) -> Sty {
        Sty {
            cramped: true,
            ..self.sup()
        }
    }
    fn num(self) -> Sty {
        let style = match self.style {
            MathStyle::Display => MathStyle::Text,
            MathStyle::Text => MathStyle::Script,
            _ => MathStyle::ScriptScript,
        };
        Sty {
            style,
            cramped: self.cramped,
        }
    }
    fn den(self) -> Sty {
        Sty {
            cramped: true,
            ..self.num()
        }
    }
    fn cramp(self) -> Sty {
        Sty { cramped: true, ..self }
    }
    fn with(self, style: MathStyle) -> Sty {
        Sty { style, ..self }
    }
}

/// A laid-out box. `h` and `d` are height above and depth below the baseline.
#[derive(Debug, Clone)]
struct BBox {
    w: f32,
    h: f32,
    d: f32,
    /// Italic correction at the right edge (glyph boxes only).
    italic: f32,
    /// `None` for kerns and explicit spaces, which do not take part in atom spacing.
    atom: Option<AtomType>,
    /// Glyph id and scale when the box is exactly one glyph (for accent attachment, kerning and script placement).
    glyph: Option<(GlyphId, f32)>,
    /// Horizontal ink extent, used to keep accents inside the box.
    ink_left: f32,
    ink_right: f32,
    /// Color override for this subtree.
    color: Option<Color>,
    /// Source range this box came from, recorded as a region when flattening.
    span: Option<(crate::ast::Span, u16)>,
    content: Content,
}

#[derive(Debug, Clone)]
enum Content {
    Empty,
    Glyph {
        id: GlyphId,
        size: f32,
    },
    Rule,
    /// Straight line from (0, -d) .. (w, h) or the other diagonal; `thickness` in px.
    Line {
        thickness: f32,
        up: bool,
    },
    List(Vec<Placed>),
}

#[derive(Debug, Clone)]
struct Placed {
    x: f32,
    /// Baseline of the child relative to the parent baseline, positive is up.
    y: f32,
    b: BBox,
}

impl BBox {
    fn empty() -> BBox {
        BBox {
            w: 0.0,
            h: 0.0,
            d: 0.0,
            italic: 0.0,
            atom: Some(AtomType::Ord),
            glyph: None,
            ink_left: 0.0,
            ink_right: 0.0,
            color: None,
            span: None,
            content: Content::Empty,
        }
    }
    fn kern(w: f32) -> BBox {
        BBox {
            w,
            atom: None,
            ..BBox::empty()
        }
    }
    fn rule(w: f32, h: f32, d: f32) -> BBox {
        BBox {
            w,
            h,
            d,
            ink_right: w,
            content: Content::Rule,
            ..BBox::empty()
        }
    }
    fn list(children: Vec<Placed>) -> BBox {
        let mut b = BBox::empty();
        b.content = Content::List(Vec::new());
        b.recompute(children);
        b
    }
    /// Sets `children` as content and grows the box to contain them.
    fn recompute(&mut self, children: Vec<Placed>) {
        let mut w: f32 = 0.0;
        let mut h: f32 = f32::NEG_INFINITY;
        let mut d: f32 = f32::NEG_INFINITY;
        let mut left: f32 = f32::INFINITY;
        let mut right: f32 = f32::NEG_INFINITY;
        for p in &children {
            w = w.max(p.x + p.b.w);
            h = h.max(p.y + p.b.h);
            d = d.max(p.b.d - p.y);
            left = left.min(p.x + p.b.ink_left);
            right = right.max(p.x + p.b.ink_right);
        }
        if children.is_empty() {
            h = 0.0;
            d = 0.0;
            left = 0.0;
            right = 0.0;
        }
        self.w = w;
        self.h = h;
        self.d = d;
        self.ink_left = left.min(0.0);
        self.ink_right = right.max(w);
        self.content = Content::List(children);
    }
    fn at(self, x: f32, y: f32) -> Placed {
        Placed { x, y, b: self }
    }
    fn with_atom(mut self, atom: AtomType) -> BBox {
        self.atom = Some(atom);
        self
    }
}

/// What `flatten` fills, plus the baseline it measures from.
struct Out<'o> {
    items: &'o mut Vec<Item>,
    regions: &'o mut Vec<crate::display::Region>,
    ascent: f32,
}

pub struct Layouter<'f, 'a> {
    font: &'f MathFont<'a>,
    base_size: f32,
    color: Color,
    line_break: Option<LineBreak>,
    hit_testing: bool,
}

impl<'f, 'a> Layouter<'f, 'a> {
    pub fn new(font: &'f MathFont<'a>, opts: &RenderOptions) -> Self {
        Layouter {
            font,
            base_size: opts.font_size,
            color: opts.color,
            line_break: opts.line_break,
            hit_testing: opts.hit_testing,
        }
    }

    pub fn layout(&self, nodes: &[Node], display_mode: bool) -> DisplayList {
        let sty = Sty {
            style: if display_mode { MathStyle::Display } else { MathStyle::Text },
            cramped: false,
        };
        let root = match self.line_break {
            Some(lb) if lb.max_width > 0.0 => self.layout_broken(nodes, sty, lb),
            _ => self.layout_list(nodes, sty),
        };
        let mut items = Vec::new();
        let mut regions = Vec::new();
        let ascent = root.h.max(0.0);
        let out = Out {
            items: &mut items,
            regions: &mut regions,
            ascent,
        };
        self.flatten(&root, -root.ink_left.min(0.0), 0.0, self.color, 0, out);
        DisplayList {
            width: root.ink_right.max(root.w) - root.ink_left.min(0.0),
            ascent,
            descent: root.d.max(0.0),
            items,
            regions,
        }
    }

    // ---- units -------------------------------------------------------------

    /// Pixels per font unit at the given style.
    fn scale(&self, sty: Sty) -> f32 {
        self.em(sty) / self.font.units_per_em()
    }

    /// Em size in pixels at the given style.
    fn em(&self, sty: Sty) -> f32 {
        let c = self.font.constants();
        let pct = match sty.style {
            MathStyle::Display | MathStyle::Text => 100.0,
            MathStyle::Script => c.script_percent_scale_down,
            MathStyle::ScriptScript => c.script_script_percent_scale_down,
        };
        self.base_size * pct / 100.0
    }

    fn mu(&self, sty: Sty) -> f32 {
        self.em(sty) / 18.0
    }

    // ---- glyph boxes -------------------------------------------------------

    fn glyph_box(&self, gid: GlyphId, sty: Sty) -> BBox {
        let s = self.scale(sty);
        let m = self.font.metrics(gid);
        BBox {
            w: m.advance * s,
            h: m.height * s,
            d: m.depth * s,
            italic: m.italic_correction * s,
            atom: Some(AtomType::Ord),
            glyph: Some((gid, s)),
            ink_left: m.x_min * s,
            ink_right: m.x_max * s,
            color: None,
            span: None,
            content: Content::Glyph {
                id: gid,
                size: self.em(sty),
            },
        }
    }

    fn resolve_glyph(&self, ch: char, variant: Variant) -> Option<GlyphId> {
        let styled = styled_char(ch, variant);
        self.font
            .glyph_index(styled)
            .or_else(|| self.font.glyph_index(styled_char(ch, Variant::Normal)))
            .or_else(|| self.font.glyph_index(ch))
    }

    /// `ssty` level for a style: 1 in script, 2 in scriptscript.
    fn ssty_level(sty: Sty) -> u8 {
        match sty.style {
            MathStyle::Script => 1,
            MathStyle::ScriptScript => 2,
            _ => 0,
        }
    }

    fn char_box(&self, ch: char, variant: Variant, sty: Sty) -> BBox {
        match self.resolve_glyph(ch, variant) {
            Some(g) => self.glyph_box(self.font.script_variant(g, Self::ssty_level(sty)), sty),
            // Missing glyph: a visible placeholder rule so the gap is noticed.
            None => BBox::rule(0.5 * self.em(sty), 0.6 * self.em(sty), 0.0),
        }
    }

    /// Smallest pre-drawn variant (or an assembly) of `gid` whose extent
    /// along the growth axis is at least `target` pixels.
    fn extensible(&self, gid: GlyphId, target: f32, sty: Sty, vertical: bool) -> BBox {
        let s = self.scale(sty);
        let variants = self.font.variants(gid, vertical);
        for &(g, adv) in &variants {
            if adv * s >= target {
                return self.glyph_box(g, sty);
            }
        }
        if let Some(parts) = self.font.assembly(gid, vertical) {
            return self.assemble(&parts, target, sty, vertical);
        }
        let largest = variants.last().map(|v| v.0).unwrap_or(gid);
        self.glyph_box(largest, sty)
    }

    fn assemble(&self, parts: &[crate::font::AssemblyPart], target: f32, sty: Sty, vertical: bool) -> BBox {
        let s = self.scale(sty);
        let min_ov = self.font.min_connector_overlap() * s;
        let has_ext = parts.iter().any(|p| p.is_extender);
        let mut seq: Vec<&crate::font::AssemblyPart> = Vec::new();
        for repeat in 1..=64usize {
            seq.clear();
            for p in parts {
                let n = if p.is_extender { repeat } else { 1 };
                for _ in 0..n {
                    seq.push(p);
                }
            }
            let sum: f32 = seq.iter().map(|p| p.full_advance * s).sum();
            let max_len = sum - (seq.len().saturating_sub(1)) as f32 * min_ov;
            if max_len >= target || !has_ext {
                break;
            }
        }
        let n = seq.len();
        let sum: f32 = seq.iter().map(|p| p.full_advance * s).sum();
        let mut overlap = if n > 1 { (sum - target) / (n - 1) as f32 } else { 0.0 };
        let max_ov = seq
            .windows(2)
            .map(|w| w[0].end_connector.min(w[1].start_connector) * s)
            .fold(f32::INFINITY, f32::min);
        overlap = overlap.max(min_ov).min(max_ov.max(min_ov));
        let mut children = Vec::new();
        let mut pos = 0.0;
        for p in &seq {
            let g = self.glyph_box(p.glyph, sty);
            if vertical {
                children.push(g.at(0.0, pos));
            } else {
                children.push(g.at(pos, 0.0));
            }
            pos += p.full_advance * s - overlap;
        }
        let total = pos + overlap;
        let mut b = BBox::list(children);
        if vertical {
            // Parts stack from the baseline upward; ink outside is clipped to the recipe.
            b.h = total;
            b.d = 0.0;
            b.w = seq.iter().map(|p| self.font.metrics(p.glyph).advance * s).fold(0.0, f32::max);
        } else {
            b.w = total;
        }
        b
    }

    /// Vertically centers a delimiter or operator on the math axis.
    fn center_on_axis(&self, b: BBox, sty: Sty) -> BBox {
        let axis = self.font.constants().axis_height * self.scale(sty);
        let shift = axis - (b.h - b.d) / 2.0;
        let mut out = BBox::list(vec![b.clone().at(0.0, shift)]);
        out.italic = b.italic;
        out.atom = b.atom;
        out.glyph = b.glyph;
        out
    }

    // ---- lists and spacing -------------------------------------------------

    fn layout_list(&self, nodes: &[Node], sty: Sty) -> BBox {
        let mut atoms = Vec::new();
        self.layout_atoms(nodes, sty, &mut atoms);
        self.hlist(atoms, sty)
    }

    /// Lays out `nodes` into `out`, splicing style and color changes inline.
    fn layout_atoms(&self, nodes: &[Node], sty: Sty, out: &mut Vec<(BBox, Sty)>) {
        let mut sty = sty;
        for n in nodes {
            match n {
                Node::Style { style, body } => {
                    sty = sty.with(*style);
                    self.layout_atoms(body, sty, out);
                }
                Node::Color { color, body } => {
                    let start = out.len();
                    self.layout_atoms(body, sty, out);
                    for (b, _) in &mut out[start..] {
                        if b.color.is_none() {
                            b.color = Some(*color);
                        }
                    }
                }
                _ => out.push((self.layout_node(n, sty), sty)),
            }
        }
    }

    /// Applies TeX's Bin/Ord rewriting and inter-atom spacing, then packs horizontally.
    fn hlist(&self, mut atoms: Vec<(BBox, Sty)>, sty: Sty) -> BBox {
        self.rewrite_atoms(&mut atoms);
        self.pack(atoms, sty)
    }

    /// TeXbook rules 5, 6 and 20: a Bin atom that cannot be binary becomes Ord.
    fn rewrite_atoms(&self, atoms: &mut [(BBox, Sty)]) {
        use AtomType::*;
        // Rules 5 and 6: a Bin that cannot be binary becomes Ord.
        let mut prev: Option<usize> = None;
        for i in 0..atoms.len() {
            let Some(cur) = atoms[i].0.atom else { continue };
            let prev_atom = prev.and_then(|p| atoms[p].0.atom);
            if cur == Bin && !matches!(prev_atom, Some(Ord | Close | Inner)) {
                atoms[i].0.atom = Some(Ord);
            } else if matches!(cur, Rel | Close | Punct) && prev_atom == Some(Bin) {
                atoms[prev.unwrap()].0.atom = Some(Ord);
            }
            prev = Some(i);
        }
        // Rule 20: the last atom cannot be Bin.
        if let Some(p) = prev {
            if atoms[p].0.atom == Some(Bin) {
                atoms[p].0.atom = Some(Ord);
            }
        }
    }

    /// Packs already-classified atoms into one horizontal box.
    fn pack(&self, atoms: Vec<(BBox, Sty)>, sty: Sty) -> BBox {
        let mut children = Vec::new();
        let mut x = 0.0;
        let mut prev_atom: Option<AtomType> = None;
        let mut last_glyph = None;
        let mut last_italic = 0.0;
        let single = atoms.len() == 1;
        for (b, asty) in atoms {
            if let (Some(l), Some(r)) = (prev_atom, b.atom) {
                x += self.spacing(l, r, asty) * self.mu(asty);
            }
            if b.atom.is_some() {
                prev_atom = b.atom;
            }
            last_glyph = b.glyph;
            last_italic = b.italic;
            let w = b.w;
            children.push(b.at(x, 0.0));
            x += w;
        }
        let mut out = BBox::list(children);
        out.w = x;
        out.atom = Some(AtomType::Ord);
        if single {
            out.glyph = last_glyph;
            out.italic = last_italic;
        }
        let _ = sty;
        out
    }

    /// Breaks the outermost list into lines no wider than `lb.max_width`.
    ///
    /// Candidates are the positions before a Rel or Bin atom, penalised as TeX
    /// penalises them in text (`\relpenalty` 500, `\binoppenalty` 700), and
    /// the split is chosen by a Knuth-Plass style dynamic program over total
    /// demerits rather than greedily, so a formula breaks into lines of even
    /// length instead of one full line and one stub. The inter-atom space at a
    /// break is discarded, exactly as TeX discards glue at a line break.
    fn layout_broken(&self, nodes: &[Node], sty: Sty, lb: LineBreak) -> BBox {
        let mut atoms = Vec::new();
        self.layout_atoms(nodes, sty, &mut atoms);
        self.rewrite_atoms(&mut atoms);
        let n = atoms.len();
        if n < 2 {
            return self.pack(atoms, sty);
        }

        // Width of each atom and the space that precedes it.
        let mut space = vec![0.0f32; n];
        let mut prev: Option<AtomType> = None;
        for (i, (b, asty)) in atoms.iter().enumerate() {
            if let (Some(l), Some(r)) = (prev, b.atom) {
                space[i] = self.spacing(l, r, *asty) * self.mu(*asty);
            }
            if b.atom.is_some() {
                prev = b.atom;
            }
        }
        // cum[i] is the width of atoms 0..i including their leading spaces.
        let mut cum = vec![0.0f32; n + 1];
        for i in 0..n {
            cum[i + 1] = cum[i] + space[i] + atoms[i].0.w;
        }
        let width = |a: usize, b: usize| cum[b] - cum[a] - space[a];

        let indent = lb.indent * self.em(sty);
        let breakable = |i: usize| -> bool {
            !matches!(
                atoms[i - 1].0.atom,
                Some(AtomType::Open) | Some(AtomType::Bin) | Some(AtomType::Rel) | None
            )
        };
        let relations: Vec<usize> = (1..n).filter(|&i| atoms[i].0.atom == Some(AtomType::Rel) && breakable(i)).collect();
        let operators: Vec<usize> = (1..n)
            .filter(|&i| matches!(atoms[i].0.atom, Some(AtomType::Rel) | Some(AtomType::Bin)) && breakable(i))
            .collect();
        if operators.is_empty() {
            return self.pack(atoms, sty);
        }

        // Knuth-Plass over total demerits: `badness` punishes a short line by
        // the cube of its slack, so lines come out even, and the atom penalty
        // breaks ties. `max_slack` rejects a solution outright, which is how
        // the relation-only pass below decides it is not good enough.
        let solve = |candidates: &[usize], max_slack: f32| -> Option<Vec<usize>> {
            let mut best = vec![f64::INFINITY; n + 1];
            let mut from = vec![0usize; n + 1];
            best[0] = 0.0;
            for &end in candidates.iter().chain(std::iter::once(&n)) {
                let penalty = if end == n {
                    0.0
                } else if atoms[end].0.atom == Some(AtomType::Rel) {
                    500.0
                } else {
                    700.0
                };
                for &start in std::iter::once(&0).chain(candidates.iter()) {
                    if start >= end || !best[start].is_finite() {
                        continue;
                    }
                    let line = width(start, end) + if start == 0 { 0.0 } else { indent };
                    let slack = lb.max_width - line;
                    let badness = if slack < 0.0 {
                        // Overfull: tolerated only for a line of one atom, which
                        // cannot be broken any further.
                        if end - start > 1 {
                            continue;
                        }
                        10_000.0
                    } else if end == n {
                        0.0 // a last line may be as short as it likes
                    } else if slack > max_slack {
                        continue;
                    } else {
                        (100.0 * slack as f64 / lb.max_width as f64).powi(3)
                    };
                    let demerits = best[start] + (10.0 + badness).powi(2) + penalty * penalty / 100.0;
                    if demerits < best[end] {
                        best[end] = demerits;
                        from[end] = start;
                    }
                }
            }
            if !best[n].is_finite() {
                return None;
            }
            let mut breaks = vec![n];
            let mut at = n;
            while at != 0 {
                at = from[at];
                breaks.push(at);
            }
            breaks.reverse();
            (breaks.len() > 2).then_some(breaks)
        };

        // A relation separates a formula at its highest level, so break there
        // when the lines still come out reasonably full, and only fall back to
        // binary operators when they do not.
        let breaks = solve(&relations, 0.45 * lb.max_width).or_else(|| solve(&operators, f32::INFINITY));
        let Some(breaks) = breaks else {
            return self.pack(atoms, sty); // nothing fits; one long line
        };

        let em = self.em(sty);
        let baselineskip = 1.2 * em;
        let lineskip = 0.1 * em;
        // amsmath adds \jot between the lines of multline and split. It is extra
        // space, so it applies whichever branch of TeX's interline rule fires,
        // which is what keeps tall lines such as stacked fractions apart.
        let jot = if sty.is_display() { 0.3 * em } else { 0.0 };
        let mut children = Vec::new();
        let mut rest = atoms;
        let mut y = 0.0f32;
        let mut prev_depth: Option<f32> = None;
        for (i, w) in breaks.windows(2).enumerate() {
            let line: Vec<(BBox, Sty)> = rest.drain(..w[1] - w[0]).collect();
            let line = self.pack(line, sty);
            if let Some(pd) = prev_depth {
                y -= if baselineskip - pd - line.h >= 0.0 {
                    baselineskip
                } else {
                    pd + line.h + lineskip
                } + jot;
            }
            prev_depth = Some(line.d);
            children.push(line.at(if i == 0 { 0.0 } else { indent }, y));
        }
        let mut out = BBox::list(children);
        out.atom = Some(AtomType::Ord);
        out
    }

    /// Inter-atom spacing in mu (TeXbook p. 170). Entries in parentheses in
    /// the book apply only in display and text style.
    fn spacing(&self, left: AtomType, right: AtomType, sty: Sty) -> f32 {
        use AtomType::*;
        let script = matches!(sty.style, MathStyle::Script | MathStyle::ScriptScript);
        let cond = |v: f32| if script { 0.0 } else { v };
        let thin = 3.0;
        let med = 4.0;
        let thick = 5.0;
        match (left, right) {
            (Ord, Op) | (Op, Ord) | (Op, Op) | (Close, Op) | (Inner, Op) => thin,
            (Ord, Bin) | (Close, Bin) | (Inner, Bin) => cond(med),
            (Bin, Ord) | (Bin, Op) | (Bin, Open) | (Bin, Inner) => cond(med),
            (Ord, Rel) | (Op, Rel) | (Close, Rel) | (Inner, Rel) => cond(thick),
            (Rel, Ord) | (Rel, Op) | (Rel, Open) | (Rel, Inner) => cond(thick),
            (Ord, Inner) | (Op, Inner) | (Close, Inner) => cond(thin),
            (Inner, Ord) | (Inner, Open) | (Inner, Punct) | (Inner, Inner) => cond(thin),
            (Punct, r) if r != Bin => cond(thin),
            _ => 0.0,
        }
    }

    // ---- nodes -------------------------------------------------------------

    fn layout_node(&self, node: &Node, sty: Sty) -> BBox {
        match node {
            Node::Symbol { ch, atom, variant } => self.char_box(*ch, *variant, sty).with_atom(*atom),
            Node::Row(nodes) => self.layout_list(nodes, sty),
            Node::Scripts { base, sup, sub } => self.layout_scripts(base, sup.as_deref(), sub.as_deref(), sty),
            Node::BigOp { ch, .. } => self.big_op(*ch, sty).with_atom(AtomType::Op),
            Node::FnName { name, .. } => self.text_box(name, Variant::Roman, sty).with_atom(AtomType::Op),
            Node::Frac {
                num,
                den,
                rule,
                style,
                delims,
            } => {
                let sty = match style {
                    Some(s) => sty.with(*s),
                    None => sty,
                };
                let f = self.fraction(num, den, *rule, sty);
                match delims {
                    None => f,
                    Some((l, r)) => self.frac_delims(f, *l, *r, sty),
                }
            }
            Node::Sqrt { radicand, index } => self.radical(radicand, index.as_deref(), sty),
            Node::LeftRight { left, body, right } => self.left_right(*left, body, *right, sty),
            Node::Middle(ch) => self.char_box(*ch, Variant::Normal, sty),
            Node::SizedDelim { ch, size, atom } => {
                // TeX: \big = \left<delim>\vbox to 8.5pt{}\right. and so on, sized by rule 19.
                let em = self.em(sty);
                let h = [0.85, 1.15, 1.45, 1.75][(*size as usize).clamp(1, 4) - 1] * em;
                let axis = self.font.constants().axis_height * self.scale(sty);
                let delta = (h - axis).max(axis);
                let target = (2.0 * delta * 0.901).max(2.0 * delta - 0.5 * em);
                match self.resolve_glyph(*ch, Variant::Normal) {
                    Some(g) => self.center_on_axis(self.extensible(g, target, sty, true), sty).with_atom(*atom),
                    None => BBox::empty().with_atom(*atom),
                }
            }
            Node::Accent { ch, base, stretchy } => self.accent(*ch, base, *stretchy, sty),
            Node::Overline(inner) => self.overline(inner, sty),
            Node::Underline(inner) => self.underline(inner, sty),
            Node::Style { style, body } => self.layout_list(body, sty.with(*style)),
            Node::Color { color, body } => {
                let mut b = self.layout_list(body, sty);
                b.color = Some(*color);
                b
            }
            Node::Text { text, variant } => self.text_box(text, *variant, sty),
            Node::Space { mu } => BBox::kern(mu * self.mu(sty)),
            Node::Array(a) => self.array(a, sty),
            Node::Phantom { body, kind } => {
                let mut b = self.layout_node(body, sty);
                match kind {
                    PhantomKind::Full => b.content = Content::Empty,
                    PhantomKind::Horizontal => {
                        b.content = Content::Empty;
                        b.h = 0.0;
                        b.d = 0.0;
                    }
                    PhantomKind::Vertical => {
                        b.content = Content::Empty;
                        b.w = 0.0;
                        b.ink_left = 0.0;
                        b.ink_right = 0.0;
                    }
                    PhantomKind::Smash => {
                        b.h = 0.0;
                        b.d = 0.0;
                    }
                }
                b
            }
            Node::OverUnder { base, over, under } => {
                // amsmath's \binrel@ gives the stack the class of its base symbol.
                let atom = Some(intrinsic_atom(base).unwrap_or(AtomType::Ord));
                let b = self.layout_node(base, sty);
                let mut out = self.limits(b, over.as_deref(), under.as_deref(), sty);
                out.atom = atom;
                out
            }
            Node::Boxed(inner) => self.boxed(inner, sty),
            Node::Cancel { body, kind } => self.cancel(body, *kind, sty),
            Node::HBrace { base, over } => self.hbrace(base, *over, sty),
            Node::XArrow { ch, over, under } => self.xarrow(*ch, over.as_deref(), under.as_deref(), sty),
            Node::Spanned { span, body } => {
                let mut b = self.layout_node(body, sty);
                if self.hit_testing {
                    b.span = Some((*span, 0));
                }
                b
            }
            Node::Class { atom, body, .. } => {
                let mut b = self.layout_node(body, sty);
                if *atom == AtomType::Op && b.glyph.is_some() {
                    b = self.center_on_axis(b, sty);
                }
                b.with_atom(*atom)
            }
        }
    }

    /// `\text{}` and function names. Upright text is shaped with the font's
    /// kerning and ligatures; other variants map char by char through the
    /// math alphabets, which have no shaping data.
    fn text_box(&self, text: &str, variant: Variant, sty: Sty) -> BBox {
        let s = self.scale(sty);
        let variant = if variant == Variant::Normal { Variant::Roman } else { variant };
        let mut children = Vec::new();
        let mut x = 0.0;
        if variant == Variant::Roman {
            for (i, word) in text.split(' ').enumerate() {
                if i > 0 {
                    x += 0.33 * self.em(sty);
                }
                for g in self.font.shape(word) {
                    if g.glyph.0 != 0 {
                        let b = self.glyph_box(g.glyph, sty);
                        children.push(b.at(x + g.x_offset * s, g.y_offset * s));
                    }
                    x += g.x_advance * s;
                }
            }
        } else {
            for ch in text.chars() {
                if ch == ' ' {
                    x += 0.33 * self.em(sty);
                    continue;
                }
                let b = self.char_box(ch, variant, sty);
                let w = b.w;
                children.push(b.at(x, 0.0));
                x += w;
            }
        }
        let mut b = BBox::list(children);
        b.w = x;
        b
    }

    fn big_op(&self, ch: char, sty: Sty) -> BBox {
        let Some(g) = self.resolve_glyph(ch, Variant::Normal) else {
            return BBox::empty();
        };
        let b = if sty.is_display() {
            let target = self.font.constants().display_operator_min_height * self.scale(sty);
            self.extensible(g, target, sty, true)
        } else {
            self.glyph_box(g, sty)
        };
        // TeX rule 13 boxes the operator, so scripts are placed by the box rule
        // (dropped from the top and bottom) rather than the single-character rule.
        let mut out = self.center_on_axis(b, sty);
        out.glyph = None;
        out
    }

    /// Math kerning between a base glyph and a script glyph (MathKernInfo).
    /// `shift` is the script baseline relative to the base baseline (positive up).
    fn script_kern(&self, base: &BBox, script: &BBox, shift: f32, top: bool) -> f32 {
        let (Some((bg, bs)), Some((sg, ss))) = (base.glyph, script.glyph) else {
            return 0.0;
        };
        if top {
            let script_bottom = shift - script.d;
            let base_top_rel_script = base.h - shift;
            self.font.math_kern(bg, KernCorner::TopRight, script_bottom / bs) * bs
                + self.font.math_kern(sg, KernCorner::BottomLeft, base_top_rel_script / ss) * ss
        } else {
            let script_top = shift + script.h;
            let base_bottom_rel_script = -base.d - shift;
            self.font.math_kern(bg, KernCorner::BottomRight, script_top / bs) * bs
                + self.font.math_kern(sg, KernCorner::TopLeft, base_bottom_rel_script / ss) * ss
        }
    }

    fn layout_scripts(&self, base: &Node, sup: Option<&Node>, sub: Option<&Node>, sty: Sty) -> BBox {
        let limits = match base {
            Node::BigOp { limits, .. } | Node::FnName { limits, .. } | Node::Class { limits, .. } => match limits {
                Limits::Limits => true,
                Limits::NoLimits => false,
                Limits::Default => sty.is_display(),
            },
            Node::HBrace { .. } => true,
            _ => false,
        };
        let base_box = self.layout_node(base, sty);
        if limits {
            return self.limits(base_box, sup, sub, sty);
        }
        let c = self.font.constants();
        let s = self.scale(sty);
        let s_sup = self.scale(sty.sup());
        let s_sub = self.scale(sty.sub());
        let is_char = base_box.glyph.is_some();
        // Rule 18a.
        let (mut u, mut v) = if is_char {
            (0.0, 0.0)
        } else {
            (
                base_box.h - c.superscript_baseline_drop_max * s_sup,
                base_box.d + c.subscript_baseline_drop_min * s_sub,
            )
        };
        let sup_box = sup.map(|n| self.layout_node(n, sty.sup()));
        let sub_box = sub.map(|n| self.layout_node(n, sty.sub()));
        let mut children = Vec::new();
        let base_w = base_box.w;
        let atom = base_box.atom.unwrap_or(AtomType::Ord);
        // Script anchors. Letters follow TeX: the superscript is pushed right by
        // the italic correction. Large operators in OpenType math fonts follow
        // the reverse convention (LuaTeX `\mathnolimitsmode=1`): the advance
        // already covers the top hook, so the subscript is pulled left instead.
        let (sup_x, sub_x) = if matches!(base, Node::BigOp { .. }) {
            (base_w, base_w - base_box.italic)
        } else {
            (base_w + base_box.italic, base_w)
        };
        let mut width = base_w;
        match (sup_box, sub_box) {
            (None, Some(sb)) => {
                // Rule 18b.
                v = v.max(c.subscript_shift_down * s).max(sb.h - c.subscript_top_max * s);
                let k = self.script_kern(&base_box, &sb, -v, false);
                children.push(base_box.at(0.0, 0.0));
                width = width.max(sub_x + k + sb.w);
                children.push(sb.at(sub_x + k, -v));
            }
            (Some(sp), None) => {
                // Rule 18c.
                let shift = if sty.cramped {
                    c.superscript_shift_up_cramped
                } else {
                    c.superscript_shift_up
                } * s;
                u = u.max(shift).max(sp.d + c.superscript_bottom_min * s);
                let k = self.script_kern(&base_box, &sp, u, true);
                children.push(base_box.at(0.0, 0.0));
                width = width.max(sup_x + k + sp.w);
                children.push(sp.at(sup_x + k, u));
            }
            (Some(sp), Some(sb)) => {
                // Rule 18d and 18e.
                let shift = if sty.cramped {
                    c.superscript_shift_up_cramped
                } else {
                    c.superscript_shift_up
                } * s;
                u = u.max(shift).max(sp.d + c.superscript_bottom_min * s);
                v = v.max(c.subscript_shift_down * s);
                let gap = (u - sp.d) - (sb.h - v);
                let gap_min = c.sub_superscript_gap_min * s;
                if gap < gap_min {
                    v += gap_min - gap;
                    let psi = c.superscript_bottom_max_with_subscript * s - (u - sp.d);
                    if psi > 0.0 {
                        u += psi;
                        v -= psi;
                    }
                }
                let ku = self.script_kern(&base_box, &sp, u, true);
                let kv = self.script_kern(&base_box, &sb, -v, false);
                children.push(base_box.at(0.0, 0.0));
                width = width.max(sup_x + ku + sp.w).max(sub_x + kv + sb.w);
                children.push(sp.at(sup_x + ku, u));
                children.push(sb.at(sub_x + kv, -v));
            }
            (None, None) => children.push(base_box.at(0.0, 0.0)),
        }
        let mut out = BBox::list(children);
        out.w = width + c.space_after_script * s;
        out.atom = Some(atom);
        out
    }

    /// Rule 13a: limits above and below a large operator.
    fn limits(&self, op: BBox, sup: Option<&Node>, sub: Option<&Node>, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let up = sup.map(|n| self.layout_node(n, sty.sup()));
        let dn = sub.map(|n| self.layout_node(n, sty.sub()));
        self.stack_limits(op, up, dn, s, c)
    }

    fn stack_limits(&self, op: BBox, up: Option<BBox>, dn: Option<BBox>, s: f32, c: &crate::font::Constants) -> BBox {
        let delta = op.italic;
        let w = [Some(op.w), up.as_ref().map(|b| b.w), dn.as_ref().map(|b| b.w)]
            .iter()
            .flatten()
            .fold(0.0, |a: f32, &b| a.max(b));
        let atom = op.atom.unwrap_or(AtomType::Op);
        let (op_h, op_d, op_w) = (op.h, op.d, op.w);
        let mut children = vec![op.at((w - op_w) / 2.0, 0.0)];
        if let Some(up) = up {
            let gap = (c.upper_limit_gap_min * s).max(c.upper_limit_baseline_rise_min * s - up.d);
            let y = op_h + gap + up.d;
            let x = (w - up.w) / 2.0 + delta / 2.0;
            children.push(up.at(x, y));
        }
        if let Some(dn) = dn {
            let gap = (c.lower_limit_gap_min * s).max(c.lower_limit_baseline_drop_min * s - dn.h);
            let y = -(op_d + gap + dn.h);
            let x = (w - dn.w) / 2.0 - delta / 2.0;
            children.push(dn.at(x, y));
        }
        let mut out = BBox::list(children);
        out.w = out.w.max(w);
        out.atom = Some(atom);
        out
    }

    /// Rule 15: generalized fractions.
    fn fraction(&self, num: &Node, den: &Node, rule: FracRule, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let n = self.layout_node(num, sty.num());
        let d = self.layout_node(den, sty.den());
        let axis = c.axis_height * s;
        let display = sty.is_display();
        let thickness = match rule {
            FracRule::Default => Some(c.fraction_rule_thickness * s),
            FracRule::None => None,
            FracRule::Custom(em) => Some(em * self.em(sty)),
        };
        let (mut u, mut v) = if thickness.is_some() {
            if display {
                (
                    c.fraction_numerator_display_style_shift_up * s,
                    c.fraction_denominator_display_style_shift_down * s,
                )
            } else {
                (c.fraction_numerator_shift_up * s, c.fraction_denominator_shift_down * s)
            }
        } else if display {
            (c.stack_top_display_style_shift_up * s, c.stack_bottom_display_style_shift_down * s)
        } else {
            (c.stack_top_shift_up * s, c.stack_bottom_shift_down * s)
        };
        let w = n.w.max(d.w);
        let mut children = Vec::new();
        if let Some(t) = thickness {
            let (num_gap, den_gap) = if display {
                (c.fraction_num_display_style_gap_min * s, c.fraction_denom_display_style_gap_min * s)
            } else {
                (c.fraction_numerator_gap_min * s, c.fraction_denominator_gap_min * s)
            };
            let num_clear = (u - n.d) - (axis + t / 2.0);
            if num_clear < num_gap {
                u += num_gap - num_clear;
            }
            let den_clear = (axis - t / 2.0) - (d.h - v);
            if den_clear < den_gap {
                v += den_gap - den_clear;
            }
            children.push(BBox::rule(w, t / 2.0, t / 2.0).at(0.0, axis));
        } else {
            let gap_min = if display { c.stack_display_style_gap_min } else { c.stack_gap_min } * s;
            let clear = (u - n.d) - (d.h - v);
            if clear < gap_min {
                let extra = (gap_min - clear) / 2.0;
                u += extra;
                v += extra;
            }
        }
        let (nw, dw) = (n.w, d.w);
        children.push(n.at((w - nw) / 2.0, u));
        children.push(d.at((w - dw) / 2.0, -v));
        let inner = BBox::list(children);
        // TeX surrounds a fraction with \nulldelimiterspace (1.2 pt, an absolute
        // dimension, so it does not shrink in script styles) on each side.
        let pad = 0.12 * self.base_size;
        let iw = inner.w;
        let mut out = BBox::list(vec![inner.at(pad, 0.0)]);
        out.w = iw + 2.0 * pad;
        out.atom = Some(AtomType::Inner);
        out
    }

    /// Rule 15e: fraction delimiters have a fixed size per style (TeX's
    /// delim1 = 2.39 em in display style, delim2 = 1.01 em otherwise).
    fn frac_delims(&self, frac: BBox, left: Delim, right: Delim, sty: Sty) -> BBox {
        let target = if sty.is_display() { 2.39 } else { 1.01 } * self.em(sty);
        let mk = |ch: Option<char>, atom: AtomType| -> BBox {
            match ch.and_then(|ch| self.resolve_glyph(ch, Variant::Normal)) {
                Some(g) => self.center_on_axis(self.extensible(g, target, sty, true), sty).with_atom(atom),
                None => BBox::kern(0.12 * self.em(sty)).with_atom(atom),
            }
        };
        let mut inner = frac;
        inner.atom = Some(AtomType::Ord);
        let all = vec![(mk(left, AtomType::Open), sty), (inner, sty), (mk(right, AtomType::Close), sty)];
        let mut out = self.hlist(all, sty);
        out.atom = Some(AtomType::Inner);
        out.glyph = None;
        out.italic = 0.0;
        out
    }

    /// Rule 11: radicals.
    fn radical(&self, radicand: &Node, index: Option<&Node>, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let body = self.layout_node(radicand, sty.cramp());
        let t = c.radical_rule_thickness * s;
        let mut clearance = if sty.is_display() {
            c.radical_display_style_vertical_gap
        } else {
            c.radical_vertical_gap
        } * s;
        let needed = body.h + body.d + clearance + t;
        let Some(sqrt_gid) = self.font.glyph_index('√') else {
            return body;
        };
        let sign = self.extensible(sqrt_gid, needed, sty, true);
        let excess = (sign.h + sign.d) - needed;
        if excess > 0.0 {
            clearance += excess / 2.0;
        }
        let rule_top = body.h + clearance + t;
        let sign_shift = rule_top - sign.h;
        let sign_w = sign.w;
        let body_w = body.w;
        let mut children = vec![
            sign.at(0.0, sign_shift),
            BBox::rule(body_w, t, 0.0).at(sign_w, rule_top - t),
            body.at(sign_w, 0.0),
        ];
        let mut out;
        if let Some(idx) = index {
            let ib = self.layout_node(idx, sty.with(MathStyle::ScriptScript));
            let kern_before = c.radical_kern_before_degree * s;
            let kern_after = c.radical_kern_after_degree * s;
            let sign_bottom = sign_shift - children[0].b.d;
            let sign_total = children[0].b.h + children[0].b.d;
            let raise = sign_bottom + sign_total * c.radical_degree_bottom_raise_percent / 100.0;
            let dx = (kern_before + ib.w + kern_after).max(0.0);
            for p in &mut children {
                p.x += dx;
            }
            children.push(ib.at(kern_before, raise));
            out = BBox::list(children);
        } else {
            out = BBox::list(children);
        }
        out.h = out.h.max(rule_top + c.radical_extra_ascender * s);
        out.atom = Some(AtomType::Ord);
        out
    }

    /// Rule 19: `\left` ... `\right`, with `\middle` delimiters sized to the same height.
    fn left_right(&self, left: Delim, body: &[Node], right: Delim, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        // Lay out the segments between \middle delimiters first to learn the height.
        let mut segments: Vec<Vec<(BBox, Sty)>> = vec![Vec::new()];
        let mut middles: Vec<char> = Vec::new();
        for n in body {
            if let Node::Middle(ch) = n {
                middles.push(*ch);
                segments.push(Vec::new());
            } else {
                self.layout_atoms(std::slice::from_ref(n), sty, segments.last_mut().unwrap());
            }
        }
        let (mut h, mut d) = (0.0f32, 0.0f32);
        for seg in &segments {
            for (b, _) in seg {
                h = h.max(b.h);
                d = d.max(b.d);
            }
        }
        let axis = c.axis_height * s;
        let delta = (h - axis).max(d + axis);
        let target = (2.0 * delta * 0.901).max(2.0 * delta - 0.5 * self.em(sty));
        let mk = |ch: Option<char>, atom: AtomType| -> BBox {
            match ch.and_then(|ch| self.resolve_glyph(ch, Variant::Normal)) {
                Some(g) => self.center_on_axis(self.extensible(g, target, sty, true), sty).with_atom(atom),
                None => BBox::kern(0.12 * self.em(sty)).with_atom(atom),
            }
        };
        let mut all = vec![(mk(left, AtomType::Open), sty)];
        for (i, seg) in segments.into_iter().enumerate() {
            if i > 0 {
                all.push((mk(Some(middles[i - 1]), AtomType::Ord), sty));
            }
            all.extend(seg);
        }
        all.push((mk(right, AtomType::Close), sty));
        let mut out = self.hlist(all, sty);
        out.atom = Some(AtomType::Inner);
        out.glyph = None;
        out.italic = 0.0;
        out
    }

    /// Rule 12: accents.
    fn accent(&self, ch: char, base: &Node, stretchy: bool, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let base_box = self.layout_node(base, sty.cramp());
        let Some(acc_gid) = self.resolve_glyph(ch, Variant::Normal) else {
            return base_box;
        };
        let acc = if stretchy {
            self.stretch_accent(acc_gid, base_box.w, sty)
        } else {
            self.glyph_box(acc_gid, sty)
        };
        // Horizontal attachment.
        let base_skew = match base_box.glyph {
            Some((g, gs)) => self.font.top_accent_attachment(g).map(|v| v * gs).unwrap_or(base_box.w / 2.0),
            None => base_box.w / 2.0,
        };
        let acc_skew = match acc.glyph {
            Some((g, gs)) => self
                .font
                .top_accent_attachment(g)
                .map(|v| v * gs)
                .unwrap_or((acc.ink_left + acc.ink_right) / 2.0),
            None => acc.w / 2.0,
        };
        let acc_x = base_skew - acc_skew;
        // Vertical placement.
        let delta = base_box.h.min(c.accent_base_height * s);
        let acc_y = base_box.h - delta;
        // TeX keeps the nucleus width: an accent may overhang on either side.
        let atom = base_box.atom.unwrap_or(AtomType::Ord);
        let bw = base_box.w;
        let italic = base_box.italic;
        let mut out = BBox::list(vec![base_box.at(0.0, 0.0), acc.at(acc_x, acc_y)]);
        out.w = bw;
        out.italic = italic;
        out.atom = Some(atom);
        out
    }

    /// Stretchy accents: the smallest variant at least as wide as the base
    /// (LuaTeX's choice for OpenType math), an assembly beyond the largest.
    fn stretch_accent(&self, gid: GlyphId, width: f32, sty: Sty) -> BBox {
        self.extensible(gid, width, sty, false)
    }

    fn overline(&self, inner: &Node, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let b = self.layout_node(inner, sty.cramp());
        let t = c.overbar_rule_thickness * s;
        let y = b.h + c.overbar_vertical_gap * s;
        let w = b.w;
        let mut out = BBox::list(vec![b.at(0.0, 0.0), BBox::rule(w, t, 0.0).at(0.0, y)]);
        out.h += c.overbar_extra_ascender * s;
        out
    }

    fn underline(&self, inner: &Node, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let b = self.layout_node(inner, sty);
        let t = c.underbar_rule_thickness * s;
        let y = -(b.d + c.underbar_vertical_gap * s + t);
        let w = b.w;
        let mut out = BBox::list(vec![b.at(0.0, 0.0), BBox::rule(w, t, 0.0).at(0.0, y)]);
        out.d += c.underbar_extra_descender * s;
        out
    }

    /// `\boxed`: content framed by a rule with \fboxsep padding.
    fn boxed(&self, inner: &Node, sty: Sty) -> BBox {
        let em = self.em(sty);
        let b = self.layout_node(inner, sty);
        let pad = 0.3 * em;
        let t = 0.04 * em;
        let (w, h, d) = (b.w + 2.0 * (pad + t), b.h + pad + t, b.d + pad + t);
        let children = vec![
            b.at(pad + t, 0.0),
            BBox::rule(w, t, 0.0).at(0.0, h - t),
            BBox::rule(w, t, 0.0).at(0.0, -d),
            BBox::rule(t, h, d).at(0.0, 0.0),
            BBox::rule(t, h, d).at(w - t, 0.0),
        ];
        let mut out = BBox::list(children);
        out.w = w;
        out
    }

    fn cancel(&self, inner: &Node, kind: CancelKind, sty: Sty) -> BBox {
        let b = self.layout_node(inner, sty);
        let t = 0.04 * self.em(sty);
        let (w, h, d) = (b.w, b.h, b.d);
        let atom = b.atom;
        let line = |up: bool| BBox {
            w,
            h,
            d,
            atom: None,
            content: Content::Line { thickness: t, up },
            ..BBox::empty()
        };
        let mut children = vec![b.at(0.0, 0.0)];
        match kind {
            CancelKind::Up => children.push(line(true).at(0.0, 0.0)),
            CancelKind::Down => children.push(line(false).at(0.0, 0.0)),
            CancelKind::Cross => {
                children.push(line(true).at(0.0, 0.0));
                children.push(line(false).at(0.0, 0.0));
            }
        }
        let mut out = BBox::list(children);
        out.atom = atom;
        out
    }

    /// `\underbrace` / `\overbrace`: a horizontal extensible brace hugging the base.
    fn hbrace(&self, base: &Node, over: bool, sty: Sty) -> BBox {
        let b = self.layout_node(base, sty);
        let ch = if over { '⏞' } else { '⏟' };
        let Some(g) = self.resolve_glyph(ch, Variant::Normal) else {
            return b;
        };
        let brace = self.extensible(g, b.w, sty, false);
        let gap = 0.1 * self.em(sty);
        let (bw, bh, bd) = (b.w, b.h, b.d);
        let bx = (bw - brace.w) / 2.0;
        let y = if over { bh + gap + brace.d } else { -(bd + gap + brace.h) };
        let mut out = BBox::list(vec![b.at(0.0, 0.0), brace.at(bx, y)]);
        out.w = out.w.max(bw);
        out.atom = Some(AtomType::Ord);
        out
    }

    /// `\xrightarrow{over}[under]`: arrow stretched to the wider label plus padding.
    fn xarrow(&self, ch: char, over: Option<&Node>, under: Option<&Node>, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let up = over.map(|n| self.layout_node(n, sty.sup()));
        let dn = under.map(|n| self.layout_node(n, sty.sub()));
        let label_w = up.as_ref().map_or(0.0, |b| b.w).max(dn.as_ref().map_or(0.0, |b| b.w));
        let Some(g) = self.resolve_glyph(ch, Variant::Normal) else {
            return BBox::empty();
        };
        let arrow = self.extensible(g, label_w + 0.8 * self.em(sty), sty, false);
        let arrow = self.center_on_axis(arrow, sty);
        let mut out = self.stack_limits(arrow, up, dn, s, c);
        out.atom = Some(AtomType::Rel);
        out
    }

    fn array(&self, a: &Array, sty: Sty) -> BBox {
        let cell_sty = Sty {
            style: a.cell_style,
            cramped: sty.cramped,
        };
        let em = self.em(cell_sty);
        // Surrounding text size, for amsmath's absolute dimensions.
        let outer = self.em(sty.with(sty.style.max(MathStyle::Text)));
        let ncols = a.cols.len().max(a.rows.iter().map(|r| r.len()).max().unwrap_or(0));
        let cells: Vec<Vec<BBox>> = a
            .rows
            .iter()
            .map(|r| r.iter().map(|c| self.layout_list(c, cell_sty)).collect())
            .collect();
        let mut col_w = vec![0.0f32; ncols];
        for r in &cells {
            for (i, b) in r.iter().enumerate() {
                col_w[i] = col_w[i].max(b.w);
            }
        }
        let is_aligned = a.cols.first() == Some(&ColAlign::Right) && a.cols.get(1) == Some(&ColAlign::Left);
        let gap = |i: usize| -> f32 {
            if a.pitch == RowPitch::SmallMatrix {
                outer / 6.0
            } else if a.pitch == RowPitch::Substack {
                0.0
            } else if is_aligned {
                if i % 2 == 1 {
                    0.0
                } else {
                    em
                }
            } else {
                em
            }
        };
        // LaTeX arrays: \@arstrut is 0.7/0.3 of \baselineskip (1.2 em), scaled by
        // \arraystretch; aligned adds \jot. amsmath's smallmatrix and substack use
        // absolute dimensions of the surrounding text size and no struts.
        let (baselineskip, lineskip, strut_h, strut_d) = match a.pitch {
            RowPitch::SmallMatrix => (0.6 * outer, 0.15 * outer, 0.0, 0.0),
            // \subarray: baselineskip = num2 + sub1 of the script font, lineskip = 3 x rule thickness.
            RowPitch::Substack => (0.38 * outer, 0.06 * outer, 0.0, 0.0),
            RowPitch::Normal => {
                let jot = if a.cell_style == MathStyle::Display { 0.3 * em } else { 0.0 };
                (1.2 * em * a.stretch + jot, 0.1 * em, 0.84 * em * a.stretch, 0.36 * em * a.stretch)
            }
        };
        let rule_t = 0.04 * em;
        // Outer padding when a vertical rule sits on the edge.
        let side = if a.pitch == RowPitch::SmallMatrix { outer / 6.0 } else { 0.0 };
        let left_pad = if a.vlines.contains(&0) { 0.5 * em } else { side };
        let right_pad = if a.vlines.contains(&ncols) { 0.5 * em } else { side };
        let mut children = Vec::new();
        let mut y = 0.0f32;
        let mut prev_d: Option<f32> = None;
        let mut row_tops = Vec::new();
        let mut row_bottoms = Vec::new();
        for (ri, r) in cells.into_iter().enumerate() {
            let rh = r.iter().map(|b| b.h).fold(strut_h, f32::max);
            let rd = r.iter().map(|b| b.d).fold(strut_d, f32::max);
            if let Some(pd) = prev_d {
                let extra = a.row_gaps.get(ri - 1).copied().unwrap_or(0.0) * em;
                let hline_extra = if a.hlines.contains(&ri) { rule_t } else { 0.0 };
                // TeX's interline glue: \baselineskip unless the boxes would touch
                // (\lineskiplimit = 0), then \lineskip between them.
                let pitch = if baselineskip - pd - rh >= 0.0 {
                    baselineskip
                } else {
                    pd + rh + lineskip
                };
                y -= pitch + extra + hline_extra;
            }
            row_tops.push(y + rh);
            row_bottoms.push(y - rd);
            let mut strut = BBox::kern(0.0);
            strut.h = strut_h;
            strut.d = strut_d;
            children.push(strut.at(left_pad, y));
            let mut x = left_pad;
            for (i, b) in r.into_iter().enumerate() {
                let align = a.cols.get(i).copied().unwrap_or(ColAlign::Center);
                let dx = match align {
                    ColAlign::Left => 0.0,
                    ColAlign::Center => (col_w[i] - b.w) / 2.0,
                    ColAlign::Right => col_w[i] - b.w,
                };
                children.push(b.at(x + dx, y));
                x += col_w[i];
                if i + 1 < ncols {
                    x += gap(i + 1);
                }
            }
            prev_d = Some(rd);
        }
        let total_w: f32 = left_pad + col_w.iter().sum::<f32>() + (1..ncols).map(gap).sum::<f32>() + right_pad;
        let top = row_tops.first().copied().unwrap_or(0.0);
        let bottom = row_bottoms.last().copied().unwrap_or(0.0);
        for &hi in &a.hlines {
            let yline = if hi == 0 {
                top
            } else if hi >= row_tops.len() {
                bottom - rule_t
            } else {
                (row_bottoms[hi - 1] + row_tops[hi]) / 2.0 - rule_t / 2.0
            };
            children.push(BBox::rule(total_w, rule_t, 0.0).at(0.0, yline));
        }
        for &vi in &a.vlines {
            let xline = if vi == 0 {
                0.0
            } else if vi >= ncols {
                total_w - rule_t
            } else {
                let before: f32 = left_pad + col_w[..vi].iter().sum::<f32>() + (1..vi).map(gap).sum::<f32>();
                before + gap(vi) / 2.0 - rule_t / 2.0
            };
            let rule_top = if a.hlines.contains(&0) { top + rule_t } else { top };
            let rule_bottom = if a.hlines.contains(&row_tops.len()) {
                bottom - rule_t
            } else {
                bottom
            };
            children.push(BBox::rule(rule_t, rule_top - rule_bottom, 0.0).at(xline, rule_bottom));
        }
        let mut out = BBox::list(children);
        out.w = out.w.max(total_w);
        // Center the whole table on the math axis.
        let axis = self.font.constants().axis_height * self.scale(sty);
        let total = out.h + out.d;
        let shift = axis - (out.h - out.d) / 2.0;
        let mut centered = BBox::list(vec![out.at(0.0, shift)]);
        centered.h = total / 2.0 + axis;
        centered.d = total / 2.0 - axis;
        centered.atom = Some(AtomType::Ord);
        centered
    }

    // ---- output ------------------------------------------------------------

    fn flatten(&self, b: &BBox, x: f32, y: f32, color: Color, depth: u16, out: Out<'_>) {
        let color = b.color.unwrap_or(color);
        let Out {
            items: out,
            regions,
            ascent,
        } = out;
        let mut depth = depth;
        if let Some((span, _)) = b.span {
            regions.push(crate::display::Region {
                start: span.start,
                end: span.end,
                x: x + b.ink_left.min(0.0),
                y: ascent - (y + b.h),
                width: (b.ink_right.max(b.w) - b.ink_left.min(0.0)).max(b.w),
                height: b.h + b.d,
                depth,
            });
            depth += 1;
        }
        match &b.content {
            Content::Empty => {}
            Content::Glyph { id, size } => out.push(Item::Glyph {
                id: id.0,
                x,
                y: ascent - y,
                size: *size,
                color,
            }),
            Content::Rule => out.push(Item::Rule {
                x,
                y: ascent - (y + b.h),
                width: b.w,
                height: b.h + b.d,
                color,
            }),
            Content::Line { thickness, up } => {
                let (y_start, y_end) = if *up {
                    (ascent - (y - b.d), ascent - (y + b.h))
                } else {
                    (ascent - (y + b.h), ascent - (y - b.d))
                };
                out.push(Item::Line {
                    x1: x,
                    y1: y_start,
                    x2: x + b.w,
                    y2: y_end,
                    thickness: *thickness,
                    color,
                });
            }
            Content::List(children) => {
                for p in children {
                    self.flatten(
                        &p.b,
                        x + p.x,
                        y + p.y,
                        color,
                        depth,
                        Out {
                            items: out,
                            regions,
                            ascent,
                        },
                    );
                }
            }
        }
    }
}

/// The atom class of a node once single-element groups are unwrapped, for
/// constructs that must keep their base's spacing class.
fn intrinsic_atom(node: &Node) -> Option<AtomType> {
    match node {
        Node::Symbol { atom, .. } => Some(*atom),
        Node::SizedDelim { atom, .. } => Some(*atom),
        Node::Class { atom, .. } => Some(*atom),
        Node::BigOp { .. } | Node::FnName { .. } => Some(AtomType::Op),
        Node::Row(v) if v.len() == 1 => intrinsic_atom(&v[0]),
        _ => None,
    }
}
