//! The subset font must lay out the whole corpus exactly like the full font.
//! Glyph ids differ after subsetting, so geometry is compared item by item.

use mathcore::{Item, MathFont, RenderOptions};

const FULL: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");
const SUBSET: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math-subset.otf");

#[path = "common/corpus.rs"]
#[allow(dead_code)] // this test only uses CORPUS
mod corpus;

/// Subsetting must keep every MATH record the engine reads, including those of
/// glyphs that are only reachable through GSUB (`ssty` alternates). fontTools
/// prunes those; `tools/subset/repair_math.py` puts them back.
#[test]
fn subset_font_keeps_every_math_record() {
    let full = MathFont::from_bytes(FULL).unwrap();
    let sub = MathFont::from_bytes(SUBSET).unwrap();
    let chars: Vec<char> = ('a'..='z')
        .chain('A'..='Z')
        .chain('0'..='9')
        .chain('α'..='ω')
        .chain('Α'..='Ω')
        .chain("∑∏∫∮⋃⋂√()[]{}⟨⟩|‖⌊⌋⌈⌉+−=<>≤≥≠∈→⇒∞∂∇".chars())
        .chain((0x1D400..=0x1D7CB).filter_map(char::from_u32))
        .collect();
    let mut checked = 0;
    for c in chars {
        let (Some(a), Some(b)) = (full.glyph_index(c), sub.glyph_index(c)) else {
            continue;
        };
        for level in 0..=2u8 {
            let (a, b) = (full.script_variant(a, level), sub.script_variant(b, level));
            checked += 1;
            assert_eq!(full.metrics(a), sub.metrics(b), "{c:?} level {level}: metrics");
            assert_eq!(
                full.top_accent_attachment(a),
                sub.top_accent_attachment(b),
                "{c:?} level {level}: accent attachment"
            );
            assert_eq!(
                full.is_extended_shape(a),
                sub.is_extended_shape(b),
                "{c:?} level {level}: extended shape"
            );
            for vertical in [true, false] {
                let (va, vb) = (full.variants(a, vertical), sub.variants(b, vertical));
                assert_eq!(
                    va.iter().map(|v| v.1).collect::<Vec<_>>(),
                    vb.iter().map(|v| v.1).collect::<Vec<_>>(),
                    "{c:?}: variants"
                );
                let (aa, ab) = (full.assembly(a, vertical), sub.assembly(b, vertical));
                assert_eq!(aa.is_some(), ab.is_some(), "{c:?}: assembly");
                if let (Some(aa), Some(ab)) = (aa, ab) {
                    assert_eq!(aa.len(), ab.len(), "{c:?}: assembly parts");
                    for (x, y) in aa.iter().zip(&ab) {
                        assert_eq!(
                            (x.start_connector, x.end_connector, x.full_advance, x.is_extender),
                            (y.start_connector, y.end_connector, y.full_advance, y.is_extender),
                            "{c:?}: assembly part"
                        );
                    }
                }
            }
        }
    }
    assert!(checked > 2000, "only {checked} glyphs checked");
    assert_eq!(full.min_connector_overlap(), sub.min_connector_overlap());
    assert_eq!(full.units_per_em(), sub.units_per_em());
}

#[test]
fn subset_font_is_geometrically_identical() {
    let full = MathFont::from_bytes(FULL).unwrap();
    let sub = MathFont::from_bytes(SUBSET).unwrap();
    assert!(SUBSET.len() < FULL.len() * 2 / 3, "subset should be materially smaller");
    let extra = [
        r"\mathsf{Ab} \mathtt{xy} \mathfrak{Cd} \mathscr{EF} \mathbb{GH} \boldsymbol{\Gamma\delta}",
        r"\alpha\beta\gamma\delta\epsilon\zeta\eta\theta\iota\kappa\lambda\mu\nu\xi\pi\rho\sigma\tau\upsilon\phi\chi\psi\omega",
        r"\forall x \in \mathbb{R}: \exists y \; x \le y \iff \neg (x > y) \land \top",
        r"\left\langle \left\lfloor \frac{a}{b} \right\rfloor \middle\| \left\lceil c \right\rceil \right\rangle",
        r"\underbrace{\overbrace{a}^{b}}_{c} \xrightarrow{d} \widetilde{efg} \overrightarrow{hi}",
        r"\Bigg( \bigg[ \Big\{ \big| x \big| \Big\} \bigg] \Bigg) \text{if fi ff} \S \P \pounds \copyright \dag",
    ];
    for (name, tex, display) in corpus::CORPUS
        .iter()
        .map(|(n, t, d)| (*n, *t, *d))
        .chain(extra.iter().map(|t| ("extra", *t, true)))
    {
        let opts = RenderOptions {
            display_mode: display,
            ..Default::default()
        };
        let a = mathcore::render(&full, tex, &opts).unwrap_or_else(|e| panic!("{name}: {e}"));
        let b = mathcore::render(&sub, tex, &opts).unwrap_or_else(|e| panic!("{name} (subset): {e}"));
        assert_eq!(a.items.len(), b.items.len(), "{name}: item count");
        assert!(
            (a.width - b.width).abs() < 1e-3 && (a.ascent - b.ascent).abs() < 1e-3,
            "{name}: size"
        );
        for (x, y) in a.items.iter().zip(&b.items) {
            match (x, y) {
                (
                    Item::Glyph {
                        id: ia,
                        x: xa,
                        y: ya,
                        size: sa,
                        ..
                    },
                    Item::Glyph {
                        id: ib,
                        x: xb,
                        y: yb,
                        size: sb,
                        ..
                    },
                ) => {
                    assert!(
                        (xa - xb).abs() < 1e-3 && (ya - yb).abs() < 1e-3 && (sa - sb).abs() < 1e-3,
                        "{name}: glyph geometry"
                    );
                    // Same outline, whatever the id.
                    assert_eq!(
                        full.metrics(ttf_parser::GlyphId(*ia)),
                        sub.metrics(ttf_parser::GlyphId(*ib)),
                        "{name}: glyph metrics"
                    );
                }
                (Item::Rule { .. }, Item::Rule { .. }) | (Item::Line { .. }, Item::Line { .. }) => assert_eq!(x, y, "{name}: rule"),
                _ => panic!("{name}: item kind differs"),
            }
        }
    }
}
