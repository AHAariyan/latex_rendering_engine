//! Layout engine: `Node` tree -> boxes -> `DisplayList`.
//!
//! This is an implementation of the TeXbook Appendix G rules with every
//! dimension taken from the font's OpenType MATH table instead of TeX's
//! fontdimens. Boxes use TeX conventions internally: the origin is on the
//! baseline at the left edge and y grows upward. `flatten` converts to the
//! y-down pixel space of the display list at the very end.

use crate::ast::*;
use crate::display::{Color, DisplayList, Item};
use crate::font::MathFont;
use crate::symbols::styled_char;
use ttf_parser::GlyphId;

#[derive(Debug, Clone)]
pub struct RenderOptions {
    /// Em size in pixels of the outermost formula.
    pub font_size: f32,
    /// `true` for `$$...$$` (display style), `false` for inline (text style).
    pub display_mode: bool,
    pub color: Color,
}

impl Default for RenderOptions {
    fn default() -> Self {
        RenderOptions {
            font_size: 32.0,
            display_mode: true,
            color: Color::BLACK,
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
    /// Glyph id when the box is exactly one glyph (for accent attachment and script placement).
    glyph: Option<(GlyphId, f32)>,
    /// Horizontal ink extent, used to keep accents inside the box.
    ink_left: f32,
    ink_right: f32,
    content: Content,
}

#[derive(Debug, Clone)]
enum Content {
    Empty,
    Glyph { id: GlyphId, size: f32 },
    Rule,
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

pub struct Layouter<'f, 'a> {
    font: &'f MathFont<'a>,
    base_size: f32,
    color: Color,
}

impl<'f, 'a> Layouter<'f, 'a> {
    pub fn new(font: &'f MathFont<'a>, opts: &RenderOptions) -> Self {
        Layouter {
            font,
            base_size: opts.font_size,
            color: opts.color,
        }
    }

    pub fn layout(&self, nodes: &[Node], display_mode: bool) -> DisplayList {
        let sty = Sty {
            style: if display_mode { MathStyle::Display } else { MathStyle::Text },
            cramped: false,
        };
        let root = self.layout_list(nodes, sty);
        let mut items = Vec::new();
        let ascent = root.h.max(0.0);
        self.flatten(&root, -root.ink_left.min(0.0), 0.0, ascent, &mut items);
        DisplayList {
            width: root.ink_right.max(root.w) - root.ink_left.min(0.0),
            ascent,
            descent: root.d.max(0.0),
            items,
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

    fn char_box(&self, ch: char, variant: Variant, sty: Sty) -> BBox {
        match self.resolve_glyph(ch, variant) {
            Some(g) => self.glyph_box(g, sty),
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

    /// Lays out `nodes` into `out`, splicing style changes inline.
    fn layout_atoms(&self, nodes: &[Node], sty: Sty, out: &mut Vec<(BBox, Sty)>) {
        let mut sty = sty;
        for n in nodes {
            match n {
                Node::Style { style, body } => {
                    sty = sty.with(*style);
                    self.layout_atoms(body, sty, out);
                }
                _ => out.push((self.layout_node(n, sty), sty)),
            }
        }
    }

    /// Applies TeX's Bin/Ord rewriting and inter-atom spacing, then packs horizontally.
    fn hlist(&self, mut atoms: Vec<(BBox, Sty)>, sty: Sty) -> BBox {
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
        out.atom = Some(Ord);
        if single {
            out.glyph = last_glyph;
            out.italic = last_italic;
        }
        let _ = sty;
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
            Node::Frac { num, den, rule, style } => {
                let sty = match style {
                    Some(s) => sty.with(*s),
                    None => sty,
                };
                self.fraction(num, den, *rule, sty)
            }
            Node::Sqrt { radicand, index } => self.radical(radicand, index.as_deref(), sty),
            Node::LeftRight { left, body, right } => self.left_right(*left, body, *right, sty),
            Node::SizedDelim { ch, size, atom } => {
                let target = [1.2, 1.8, 2.4, 3.0][(*size as usize).clamp(1, 4) - 1] * self.em(sty);
                match self.resolve_glyph(*ch, Variant::Normal) {
                    Some(g) => self.center_on_axis(self.extensible(g, target, sty, true), sty).with_atom(*atom),
                    None => BBox::empty().with_atom(*atom),
                }
            }
            Node::Accent { ch, base, stretchy } => self.accent(*ch, base, *stretchy, sty),
            Node::Overline(inner) => self.overline(inner, sty),
            Node::Underline(inner) => self.underline(inner, sty),
            Node::Style { style, body } => self.layout_list(body, sty.with(*style)),
            Node::Text { text, variant } => self.text_box(text, *variant, sty),
            Node::Space { mu } => BBox::kern(mu * self.mu(sty)),
            Node::Array { rows, cols, cell_style } => self.array(rows, cols, *cell_style, sty),
            Node::Phantom(inner) => {
                let mut b = self.layout_node(inner, sty);
                b.content = Content::Empty;
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
        }
    }

    fn text_box(&self, text: &str, variant: Variant, sty: Sty) -> BBox {
        let mut children = Vec::new();
        let mut x = 0.0;
        for ch in text.chars() {
            if ch == ' ' {
                x += 0.33 * self.em(sty);
                continue;
            }
            let b = self.char_box(ch, if variant == Variant::Normal { Variant::Roman } else { variant }, sty);
            let w = b.w;
            children.push(b.at(x, 0.0));
            x += w;
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

    fn layout_scripts(&self, base: &Node, sup: Option<&Node>, sub: Option<&Node>, sty: Sty) -> BBox {
        let limits = match base {
            Node::BigOp { limits, .. } | Node::FnName { limits, .. } => match limits {
                Limits::Limits => true,
                Limits::NoLimits => false,
                Limits::Default => sty.is_display(),
            },
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
        children.push(base_box.at(0.0, 0.0));
        let mut width = base_w;
        match (sup_box, sub_box) {
            (None, Some(sb)) => {
                // Rule 18b.
                v = v.max(c.subscript_shift_down * s).max(sb.h - c.subscript_top_max * s);
                width = width.max(sub_x + sb.w);
                children.push(sb.at(sub_x, -v));
            }
            (Some(sp), None) => {
                // Rule 18c.
                let shift = if sty.cramped {
                    c.superscript_shift_up_cramped
                } else {
                    c.superscript_shift_up
                } * s;
                u = u.max(shift).max(sp.d + c.superscript_bottom_min * s);
                width = width.max(sup_x + sp.w);
                children.push(sp.at(sup_x, u));
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
                width = width.max(sup_x + sp.w).max(sub_x + sb.w);
                children.push(sp.at(sup_x, u));
                children.push(sb.at(sub_x, -v));
            }
            (None, None) => {}
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
    fn fraction(&self, num: &Node, den: &Node, rule: bool, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let n = self.layout_node(num, sty.num());
        let d = self.layout_node(den, sty.den());
        let axis = c.axis_height * s;
        let display = sty.is_display();
        let (mut u, mut v) = if rule {
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
        if rule {
            let t = c.fraction_rule_thickness * s;
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
        // TeX surrounds a fraction with \nulldelimiterspace on each side.
        let pad = 0.12 * self.em(sty);
        let iw = inner.w;
        let mut out = BBox::list(vec![inner.at(pad, 0.0)]);
        out.w = iw + 2.0 * pad;
        out.atom = Some(AtomType::Inner);
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
            let dx = kern_before + ib.w + kern_after;
            let dx = dx.max(0.0);
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

    /// Rule 19: `\left` ... `\right`.
    fn left_right(&self, left: Delim, body: &[Node], right: Delim, sty: Sty) -> BBox {
        let c = self.font.constants();
        let s = self.scale(sty);
        let mut atoms = Vec::new();
        self.layout_atoms(body, sty, &mut atoms);
        let (mut h, mut d) = (0.0f32, 0.0f32);
        for (b, _) in &atoms {
            h = h.max(b.h);
            d = d.max(b.d);
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
        all.extend(atoms);
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
            self.extensible(acc_gid, base_box.w, sty, false)
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
        let mut acc_x = base_skew - acc_skew;
        // Vertical placement.
        let delta = base_box.h.min(c.accent_base_height * s);
        let acc_y = base_box.h - delta;
        let mut base_x = 0.0;
        let left_overhang = acc_x + acc.ink_left;
        if left_overhang < 0.0 {
            base_x -= left_overhang;
            acc_x -= left_overhang;
        }
        let atom = base_box.atom.unwrap_or(AtomType::Ord);
        let bw = base_box.w;
        let italic = base_box.italic;
        let mut out = BBox::list(vec![base_box.at(base_x, 0.0), acc.at(acc_x, acc_y)]);
        out.w = out.w.max(base_x + bw);
        out.italic = italic;
        out.atom = Some(atom);
        out
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

    fn array(&self, rows: &[Vec<Vec<Node>>], cols: &[ColAlign], cell_style: MathStyle, sty: Sty) -> BBox {
        let cell_sty = Sty {
            style: cell_style.min(sty.style.max(MathStyle::Text)),
            cramped: sty.cramped,
        };
        let cell_sty = if cell_style == MathStyle::Script || cell_style == MathStyle::Display {
            Sty {
                style: cell_style,
                cramped: sty.cramped,
            }
        } else {
            cell_sty
        };
        let em = self.em(sty);
        let ncols = cols.len().max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
        let cells: Vec<Vec<BBox>> = rows
            .iter()
            .map(|r| r.iter().map(|c| self.layout_list(c, cell_sty)).collect())
            .collect();
        let mut col_w = vec![0.0f32; ncols];
        for r in &cells {
            for (i, b) in r.iter().enumerate() {
                col_w[i] = col_w[i].max(b.w);
            }
        }
        let is_aligned = cols.first() == Some(&ColAlign::Right) && cols.get(1) == Some(&ColAlign::Left);
        let gap = |i: usize| -> f32 {
            if is_aligned {
                if i % 2 == 1 {
                    0.0
                } else {
                    1.0 * em
                }
            } else {
                1.0 * em
            }
        };
        let baselineskip = 1.2 * em + if cell_style == MathStyle::Display { 0.3 * em } else { 0.0 };
        let lineskip = 0.1 * em;
        let mut children = Vec::new();
        let mut y = 0.0f32;
        let mut prev_d: Option<f32> = None;
        // Every row carries a \strut so rows of short content still get TeX's line pitch.
        let strut_h = 0.7 * self.em(cell_sty);
        let strut_d = 0.3 * self.em(cell_sty);
        for r in cells {
            let rh = r.iter().map(|b| b.h).fold(strut_h, f32::max);
            let rd = r.iter().map(|b| b.d).fold(strut_d, f32::max);
            if let Some(pd) = prev_d {
                y -= (pd + rh + lineskip).max(baselineskip);
            }
            let mut strut = BBox::kern(0.0);
            strut.h = strut_h;
            strut.d = strut_d;
            children.push(strut.at(0.0, y));
            let mut x = 0.0;
            for (i, b) in r.into_iter().enumerate() {
                let align = cols.get(i).copied().unwrap_or(ColAlign::Center);
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
        let mut out = BBox::list(children);
        let total_w: f32 = col_w.iter().sum::<f32>() + (1..ncols).map(gap).sum::<f32>();
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

    fn flatten(&self, b: &BBox, x: f32, y: f32, ascent: f32, out: &mut Vec<Item>) {
        match &b.content {
            Content::Empty => {}
            Content::Glyph { id, size } => out.push(Item::Glyph {
                id: id.0,
                x,
                y: ascent - y,
                size: *size,
                color: self.color,
            }),
            Content::Rule => out.push(Item::Rule {
                x,
                y: ascent - (y + b.h),
                width: b.w,
                height: b.h + b.d,
                color: self.color,
            }),
            Content::List(children) => {
                for p in children {
                    self.flatten(&p.b, x + p.x, y + p.y, ascent, out);
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
        Node::BigOp { .. } | Node::FnName { .. } => Some(AtomType::Op),
        Node::Row(v) if v.len() == 1 => intrinsic_atom(&v[0]),
        _ => None,
    }
}
