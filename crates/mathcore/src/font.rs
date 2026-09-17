//! Access to an OpenType math font: glyph metrics, the MATH constants,
//! italic corrections, accent attachment points, size variants and
//! extensible glyph assemblies.
//!
//! All values are in font units. The layout engine scales them.

use crate::error::{Error, Result};
use ttf_parser::{math, Face, GlyphId, OutlineBuilder};

/// MATH table constants, extracted once into plain floats (font units).
#[derive(Debug, Clone)]
pub struct Constants {
    pub script_percent_scale_down: f32,
    pub script_script_percent_scale_down: f32,
    pub delimited_sub_formula_min_height: f32,
    pub display_operator_min_height: f32,
    pub math_leading: f32,
    pub axis_height: f32,
    pub accent_base_height: f32,
    pub flattened_accent_base_height: f32,
    pub subscript_shift_down: f32,
    pub subscript_top_max: f32,
    pub subscript_baseline_drop_min: f32,
    pub superscript_shift_up: f32,
    pub superscript_shift_up_cramped: f32,
    pub superscript_bottom_min: f32,
    pub superscript_baseline_drop_max: f32,
    pub sub_superscript_gap_min: f32,
    pub superscript_bottom_max_with_subscript: f32,
    pub space_after_script: f32,
    pub upper_limit_gap_min: f32,
    pub upper_limit_baseline_rise_min: f32,
    pub lower_limit_gap_min: f32,
    pub lower_limit_baseline_drop_min: f32,
    pub stack_top_shift_up: f32,
    pub stack_top_display_style_shift_up: f32,
    pub stack_bottom_shift_down: f32,
    pub stack_bottom_display_style_shift_down: f32,
    pub stack_gap_min: f32,
    pub stack_display_style_gap_min: f32,
    pub fraction_numerator_shift_up: f32,
    pub fraction_numerator_display_style_shift_up: f32,
    pub fraction_denominator_shift_down: f32,
    pub fraction_denominator_display_style_shift_down: f32,
    pub fraction_numerator_gap_min: f32,
    pub fraction_num_display_style_gap_min: f32,
    pub fraction_rule_thickness: f32,
    pub fraction_denominator_gap_min: f32,
    pub fraction_denom_display_style_gap_min: f32,
    pub overbar_vertical_gap: f32,
    pub overbar_rule_thickness: f32,
    pub overbar_extra_ascender: f32,
    pub underbar_vertical_gap: f32,
    pub underbar_rule_thickness: f32,
    pub underbar_extra_descender: f32,
    pub radical_vertical_gap: f32,
    pub radical_display_style_vertical_gap: f32,
    pub radical_rule_thickness: f32,
    pub radical_extra_ascender: f32,
    pub radical_kern_before_degree: f32,
    pub radical_kern_after_degree: f32,
    pub radical_degree_bottom_raise_percent: f32,
}

/// Glyph metrics in font units, y grows upward from the baseline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphMetrics {
    pub advance: f32,
    /// Ink extent above the baseline (may be negative for glyphs entirely below it).
    pub height: f32,
    /// Ink extent below the baseline (positive when ink descends).
    pub depth: f32,
    pub x_min: f32,
    pub x_max: f32,
    pub italic_correction: f32,
}

/// Corner of a glyph for math kerning.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernCorner {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

/// One piece of an extensible glyph assembly.
#[derive(Debug, Clone, Copy)]
pub struct AssemblyPart {
    pub glyph: GlyphId,
    pub start_connector: f32,
    pub end_connector: f32,
    pub full_advance: f32,
    pub is_extender: bool,
}

pub struct MathFont<'a> {
    face: Face<'a>,
    upem: f32,
    consts: Constants,
    x_height: f32,
}

/// One shaped glyph of a text run, in font units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph {
    pub glyph: GlyphId,
    pub x_advance: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

impl<'a> MathFont<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Result<Self> {
        let face = Face::parse(data, 0).map_err(|e| Error::Font(format!("cannot parse font: {e}")))?;
        let table = face.tables().math.ok_or_else(|| Error::Font("font has no MATH table".into()))?;
        let c = table.constants.ok_or_else(|| Error::Font("MATH table has no constants".into()))?;
        let v = |m: math::MathValue| m.value as f32;
        let consts = Constants {
            script_percent_scale_down: c.script_percent_scale_down() as f32,
            script_script_percent_scale_down: c.script_script_percent_scale_down() as f32,
            delimited_sub_formula_min_height: c.delimited_sub_formula_min_height() as f32,
            display_operator_min_height: c.display_operator_min_height() as f32,
            math_leading: v(c.math_leading()),
            axis_height: v(c.axis_height()),
            accent_base_height: v(c.accent_base_height()),
            flattened_accent_base_height: v(c.flattened_accent_base_height()),
            subscript_shift_down: v(c.subscript_shift_down()),
            subscript_top_max: v(c.subscript_top_max()),
            subscript_baseline_drop_min: v(c.subscript_baseline_drop_min()),
            superscript_shift_up: v(c.superscript_shift_up()),
            superscript_shift_up_cramped: v(c.superscript_shift_up_cramped()),
            superscript_bottom_min: v(c.superscript_bottom_min()),
            superscript_baseline_drop_max: v(c.superscript_baseline_drop_max()),
            sub_superscript_gap_min: v(c.sub_superscript_gap_min()),
            superscript_bottom_max_with_subscript: v(c.superscript_bottom_max_with_subscript()),
            space_after_script: v(c.space_after_script()),
            upper_limit_gap_min: v(c.upper_limit_gap_min()),
            upper_limit_baseline_rise_min: v(c.upper_limit_baseline_rise_min()),
            lower_limit_gap_min: v(c.lower_limit_gap_min()),
            lower_limit_baseline_drop_min: v(c.lower_limit_baseline_drop_min()),
            stack_top_shift_up: v(c.stack_top_shift_up()),
            stack_top_display_style_shift_up: v(c.stack_top_display_style_shift_up()),
            stack_bottom_shift_down: v(c.stack_bottom_shift_down()),
            stack_bottom_display_style_shift_down: v(c.stack_bottom_display_style_shift_down()),
            stack_gap_min: v(c.stack_gap_min()),
            stack_display_style_gap_min: v(c.stack_display_style_gap_min()),
            fraction_numerator_shift_up: v(c.fraction_numerator_shift_up()),
            fraction_numerator_display_style_shift_up: v(c.fraction_numerator_display_style_shift_up()),
            fraction_denominator_shift_down: v(c.fraction_denominator_shift_down()),
            fraction_denominator_display_style_shift_down: v(c.fraction_denominator_display_style_shift_down()),
            fraction_numerator_gap_min: v(c.fraction_numerator_gap_min()),
            fraction_num_display_style_gap_min: v(c.fraction_num_display_style_gap_min()),
            fraction_rule_thickness: v(c.fraction_rule_thickness()),
            fraction_denominator_gap_min: v(c.fraction_denominator_gap_min()),
            fraction_denom_display_style_gap_min: v(c.fraction_denom_display_style_gap_min()),
            overbar_vertical_gap: v(c.overbar_vertical_gap()),
            overbar_rule_thickness: v(c.overbar_rule_thickness()),
            overbar_extra_ascender: v(c.overbar_extra_ascender()),
            underbar_vertical_gap: v(c.underbar_vertical_gap()),
            underbar_rule_thickness: v(c.underbar_rule_thickness()),
            underbar_extra_descender: v(c.underbar_extra_descender()),
            radical_vertical_gap: v(c.radical_vertical_gap()),
            radical_display_style_vertical_gap: v(c.radical_display_style_vertical_gap()),
            radical_rule_thickness: v(c.radical_rule_thickness()),
            radical_extra_ascender: v(c.radical_extra_ascender()),
            radical_kern_before_degree: v(c.radical_kern_before_degree()),
            radical_kern_after_degree: v(c.radical_kern_after_degree()),
            radical_degree_bottom_raise_percent: c.radical_degree_bottom_raise_percent() as f32,
        };
        let upem = face.units_per_em() as f32;
        let x_height = face.x_height().map(|x| x as f32).unwrap_or(upem * 0.45);
        Ok(MathFont {
            face,
            upem,
            consts,
            x_height,
        })
    }

    pub fn units_per_em(&self) -> f32 {
        self.upem
    }

    pub fn x_height(&self) -> f32 {
        self.x_height
    }

    pub fn constants(&self) -> &Constants {
        &self.consts
    }

    pub fn face(&self) -> &Face<'a> {
        &self.face
    }

    pub fn glyph_index(&self, ch: char) -> Option<GlyphId> {
        self.face.glyph_index(ch).filter(|g| g.0 != 0)
    }

    pub fn metrics(&self, gid: GlyphId) -> GlyphMetrics {
        let advance = self.face.glyph_hor_advance(gid).unwrap_or(0) as f32;
        let (x_min, x_max, height, depth) = match self.face.glyph_bounding_box(gid) {
            Some(r) => (r.x_min as f32, r.x_max as f32, r.y_max as f32, -(r.y_min as f32)),
            None => (0.0, advance, 0.0, 0.0),
        };
        GlyphMetrics {
            advance,
            height,
            depth,
            x_min,
            x_max,
            italic_correction: self.italic_correction(gid),
        }
    }

    fn math_table(&self) -> Option<math::Table<'a>> {
        self.face.tables().math
    }

    pub fn italic_correction(&self, gid: GlyphId) -> f32 {
        self.math_table()
            .and_then(|t| t.glyph_info)
            .and_then(|g| g.italic_corrections)
            .and_then(|ic| ic.get(gid))
            .map(|v| v.value as f32)
            .unwrap_or(0.0)
    }

    /// Horizontal position of the accent attachment point, if the font defines one.
    pub fn top_accent_attachment(&self, gid: GlyphId) -> Option<f32> {
        self.math_table()
            .and_then(|t| t.glyph_info)
            .and_then(|g| g.top_accent_attachments)
            .and_then(|ta| ta.get(gid))
            .map(|v| v.value as f32)
    }

    pub fn is_extended_shape(&self, gid: GlyphId) -> bool {
        self.math_table()
            .and_then(|t| t.glyph_info)
            .and_then(|g| g.extended_shapes)
            .map(|cov| cov.get(gid).is_some())
            .unwrap_or(false)
    }

    pub fn min_connector_overlap(&self) -> f32 {
        self.math_table()
            .and_then(|t| t.variants)
            .map(|v| v.min_connector_overlap as f32)
            .unwrap_or(0.0)
    }

    fn construction(&self, gid: GlyphId, vertical: bool) -> Option<math::GlyphConstruction<'a>> {
        let variants = self.math_table()?.variants?;
        let table = if vertical {
            variants.vertical_constructions
        } else {
            variants.horizontal_constructions
        };
        table.get(gid)
    }

    /// Pre-drawn size variants of a glyph, smallest first, including the base glyph.
    /// Each entry is `(glyph, advance along the growth axis)`.
    pub fn variants(&self, gid: GlyphId, vertical: bool) -> Vec<(GlyphId, f32)> {
        let mut out = Vec::new();
        if let Some(c) = self.construction(gid, vertical) {
            for v in c.variants {
                out.push((v.variant_glyph, v.advance_measurement as f32));
            }
        }
        if out.is_empty() {
            let m = self.metrics(gid);
            out.push((gid, if vertical { m.height + m.depth } else { m.advance }));
        }
        out
    }

    /// Extensible assembly recipe for a glyph, bottom-to-top (or left-to-right).
    pub fn assembly(&self, gid: GlyphId, vertical: bool) -> Option<Vec<AssemblyPart>> {
        let asm = self.construction(gid, vertical)?.assembly?;
        let parts: Vec<AssemblyPart> = asm
            .parts
            .into_iter()
            .map(|p| AssemblyPart {
                glyph: p.glyph_id,
                start_connector: p.start_connector_length as f32,
                end_connector: p.end_connector_length as f32,
                full_advance: p.full_advance as f32,
                is_extender: p.part_flags.extender(),
            })
            .collect();
        if parts.is_empty() {
            None
        } else {
            Some(parts)
        }
    }

    /// Math kerning (MathKernInfo): the kern to apply between a glyph and a
    /// script at the given correction height (font units, relative to this
    /// glyph's baseline). Returns 0 when the font defines none.
    pub fn math_kern(&self, gid: GlyphId, corner: KernCorner, height: f32) -> f32 {
        let Some(info) = self
            .math_table()
            .and_then(|t| t.glyph_info)
            .and_then(|g| g.kern_infos)
            .and_then(|k| k.get(gid))
        else {
            return 0.0;
        };
        let kern = match corner {
            KernCorner::TopRight => info.top_right,
            KernCorner::TopLeft => info.top_left,
            KernCorner::BottomRight => info.bottom_right,
            KernCorner::BottomLeft => info.bottom_left,
        };
        let Some(kern) = kern else { return 0.0 };
        let count = kern.count();
        let mut i = 0u16;
        while i < count {
            match kern.height(i) {
                Some(h) if height >= h.value as f32 => i += 1,
                _ => break,
            }
        }
        kern.kern(i).map(|v| v.value as f32).unwrap_or(0.0)
    }

    /// Shapes a run of text left to right with the font's `liga` ligatures and
    /// `kern` pair adjustments. This is a deliberately small shaper: the math
    /// fonts in scope are Latin and these two features are what `\text{}`
    /// needs. Characters the font lacks come back as glyph 0.
    pub fn shape(&self, text: &str) -> Vec<ShapedGlyph> {
        let mut glyphs: Vec<GlyphId> = text.chars().map(|c| self.face.glyph_index(c).unwrap_or(GlyphId(0))).collect();
        self.apply_ligatures(&mut glyphs);
        let mut out = Vec::with_capacity(glyphs.len());
        for (i, &g) in glyphs.iter().enumerate() {
            let mut x_advance = self.face.glyph_hor_advance(g).unwrap_or(0) as f32;
            if let Some(&next) = glyphs.get(i + 1) {
                x_advance += self.pair_kern(g, next);
            }
            out.push(ShapedGlyph {
                glyph: g,
                x_advance,
                x_offset: 0.0,
                y_offset: 0.0,
            });
        }
        out
    }

    /// Lookup indices of every feature with the given tag.
    fn feature_lookups(table: &ttf_parser::opentype_layout::LayoutTable<'_>, tag: &[u8; 4]) -> Vec<u16> {
        let tag = ttf_parser::Tag::from_bytes(tag);
        let mut out = Vec::new();
        for i in 0..table.features.len() {
            if let Some(f) = table.features.get(i) {
                if f.tag == tag {
                    out.extend(f.lookup_indices);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// GPOS `kern` adjustment to the advance of `first` when followed by `second`.
    fn pair_kern(&self, first: GlyphId, second: GlyphId) -> f32 {
        use ttf_parser::gpos::{PairAdjustment, PositioningSubtable};
        let Some(gpos) = self.face.tables().gpos else { return 0.0 };
        for li in Self::feature_lookups(&gpos, b"kern") {
            let Some(lookup) = gpos.lookups.get(li) else { continue };
            for sub in lookup.subtables.into_iter::<PositioningSubtable>() {
                let PositioningSubtable::Pair(pair) = sub else { continue };
                match pair {
                    PairAdjustment::Format1 { coverage, sets } => {
                        let Some(idx) = coverage.get(first) else { continue };
                        if let Some(v) = sets.get(idx).and_then(|set| set.get(second)) {
                            return v.0.x_advance as f32;
                        }
                    }
                    PairAdjustment::Format2 { coverage, classes, matrix } => {
                        if coverage.get(first).is_none() {
                            continue;
                        }
                        if let Some(v) = matrix.get((classes.0.get(first), classes.1.get(second))) {
                            return v.0.x_advance as f32;
                        }
                    }
                }
            }
        }
        0.0
    }

    /// OpenType MATH `ssty` alternate for script (level 1) or scriptscript
    /// (level 2) style, or the glyph itself when the font has none. Fonts use
    /// these for glyphs whose script-size form needs different proportions.
    pub fn script_variant(&self, gid: GlyphId, level: u8) -> GlyphId {
        use ttf_parser::gsub::{SingleSubstitution, SubstitutionSubtable};
        if level == 0 {
            return gid;
        }
        let Some(gsub) = self.face.tables().gsub else { return gid };
        for li in Self::feature_lookups(&gsub, b"ssty") {
            let Some(lookup) = gsub.lookups.get(li) else { continue };
            for sub in lookup.subtables.into_iter::<SubstitutionSubtable>() {
                match sub {
                    SubstitutionSubtable::Single(SingleSubstitution::Format1 { coverage, delta }) if coverage.get(gid).is_some() => {
                        return GlyphId((gid.0 as i32 + delta as i32) as u16);
                    }
                    SubstitutionSubtable::Single(SingleSubstitution::Format2 { coverage, substitutes }) => {
                        if let Some(idx) = coverage.get(gid) {
                            if let Some(g) = substitutes.get(idx) {
                                return g;
                            }
                        }
                    }
                    SubstitutionSubtable::Alternate(alt) => {
                        if let Some(idx) = alt.coverage.get(gid) {
                            if let Some(set) = alt.alternate_sets.get(idx) {
                                let n = set.alternates.len();
                                if n > 0 {
                                    let pick = (level as u16 - 1).min(n - 1);
                                    if let Some(g) = set.alternates.get(pick) {
                                        return g;
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        gid
    }

    /// Applies GSUB `liga` substitutions in place.
    fn apply_ligatures(&self, glyphs: &mut Vec<GlyphId>) {
        use ttf_parser::gsub::SubstitutionSubtable;
        let Some(gsub) = self.face.tables().gsub else { return };
        for li in Self::feature_lookups(&gsub, b"liga") {
            let Some(lookup) = gsub.lookups.get(li) else { continue };
            for sub in lookup.subtables.into_iter::<SubstitutionSubtable>() {
                let SubstitutionSubtable::Ligature(lig) = sub else { continue };
                let mut i = 0;
                while i < glyphs.len() {
                    let Some(idx) = lig.coverage.get(glyphs[i]) else {
                        i += 1;
                        continue;
                    };
                    let Some(set) = lig.ligature_sets.get(idx) else {
                        i += 1;
                        continue;
                    };
                    let mut replaced = false;
                    for k in 0..set.len() {
                        let Some(l) = set.get(k) else { continue };
                        let n = l.components.len() as usize;
                        if i + n < glyphs.len() && l.components.into_iter().enumerate().all(|(c, g)| glyphs[i + 1 + c] == g) {
                            glyphs.splice(i..=i + n, [l.glyph]);
                            replaced = true;
                            break;
                        }
                    }
                    i += 1;
                    let _ = replaced;
                }
            }
        }
    }

    /// Feeds the glyph outline (font units, y up) to `builder`.
    pub fn outline(&self, gid: GlyphId, builder: &mut dyn OutlineBuilder) -> bool {
        self.face.outline_glyph(gid, builder).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(crate) const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");

    #[test]
    fn loads_latin_modern_math() {
        let f = MathFont::from_bytes(FONT).unwrap();
        assert_eq!(f.units_per_em(), 1000.0);
        let c = f.constants();
        assert!(c.axis_height > 200.0 && c.axis_height < 300.0, "axis {}", c.axis_height);
        assert!(c.fraction_rule_thickness > 0.0);
        assert!(c.script_percent_scale_down > 50.0);
        let x = f.glyph_index('x').unwrap();
        let m = f.metrics(x);
        assert!(m.advance > 0.0 && m.height > 300.0 && m.height < 600.0, "{m:?}");
        assert!(m.depth.abs() < 30.0);
    }

    #[test]
    fn radical_has_variants_and_assembly() {
        let f = MathFont::from_bytes(FONT).unwrap();
        let sqrt = f.glyph_index('√').unwrap();
        let vars = f.variants(sqrt, true);
        assert!(vars.len() >= 3, "{vars:?}");
        assert!(vars.windows(2).all(|w| w[0].1 <= w[1].1));
        assert!(f.assembly(sqrt, true).is_some());
        let paren = f.glyph_index('(').unwrap();
        assert!(f.assembly(paren, true).unwrap().iter().any(|p| p.is_extender));
    }

    #[test]
    fn math_kerning_is_zero_without_kern_info() {
        // Latin Modern Math ships no MathKernInfo; the lookup must degrade to 0.
        let f = MathFont::from_bytes(FONT).unwrap();
        let fi = f.glyph_index('𝑓').unwrap();
        assert_eq!(f.math_kern(fi, KernCorner::BottomRight, 0.0), 0.0);
    }

    #[test]
    fn stix_two_has_math_kerning() {
        const STIX: &[u8] = include_bytes!("../../../assets/fonts/STIXTwoMath-Regular.otf");
        let f = MathFont::from_bytes(STIX).unwrap();
        let fi = f.glyph_index('𝑓').unwrap();
        // STIX Two kerns subscripts under the hook of italic f.
        assert!(f.math_kern(fi, KernCorner::BottomRight, 0.0) < 0.0);
        let v = f.glyph_index('𝑉').unwrap();
        assert!(f.math_kern(v, KernCorner::BottomRight, 0.0) < 0.0);
    }

    #[test]
    fn all_bundled_fonts_load() {
        for bytes in [
            &include_bytes!("../../../assets/fonts/STIXTwoMath-Regular.otf")[..],
            &include_bytes!("../../../assets/fonts/LibertinusMath-Regular.otf")[..],
        ] {
            let f = MathFont::from_bytes(bytes).unwrap();
            assert!(f.constants().axis_height > 0.0);
            assert!(f.glyph_index('√').is_some());
            assert!(f.variants(f.glyph_index('(').unwrap(), true).len() > 1);
        }
    }

    #[test]
    fn ssty_alternates_exist_for_script_styles() {
        let f = MathFont::from_bytes(FONT).unwrap();
        let i = f.glyph_index('𝑖').unwrap();
        let s1 = f.script_variant(i, 1);
        assert_ne!(s1, i, "Latin Modern Math has an ssty alternate for math italic i");
        assert_eq!(f.script_variant(i, 0), i);
        let x = f.glyph_index('x').unwrap();
        let _ = f.script_variant(x, 2); // must not panic for glyphs without alternates
    }

    #[test]
    fn shaping_applies_kerning_and_ligatures() {
        // Latin Modern Math carries no kerning for upright Latin, so use the other fonts.
        let stix = MathFont::from_bytes(include_bytes!("../../../assets/fonts/STIXTwoMath-Regular.otf")).unwrap();
        let av = stix.shape("AV");
        assert_eq!(av.len(), 2);
        let plain: f32 = ["A", "V"].iter().map(|c| stix.shape(c)[0].x_advance).sum();
        let shaped: f32 = av.iter().map(|g| g.x_advance).sum();
        assert!(shaped < plain, "AV must be kerned: {shaped} vs {plain}");
        let lib = MathFont::from_bytes(include_bytes!("../../../assets/fonts/LibertinusMath-Regular.otf")).unwrap();
        assert_eq!(lib.shape("fi").len(), 1, "fi ligature");
        let lm = MathFont::from_bytes(FONT).unwrap();
        assert_eq!(lm.shape("otherwise").len(), 9);
    }

    #[test]
    fn italic_correction_and_accent_attachment() {
        let f = MathFont::from_bytes(FONT).unwrap();
        let fi = f.glyph_index('𝑓').unwrap();
        assert!(f.italic_correction(fi) > 0.0);
        assert!(f.top_accent_attachment(fi).is_some());
    }
}
