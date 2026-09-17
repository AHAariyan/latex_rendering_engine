//! `mathcore`: a native TeX math typesetting engine.
//!
//! Pipeline: `parse` (TeX -> AST) -> `Layouter` (AST + OpenType MATH font ->
//! boxes) -> `DisplayList` (glyph ids, positions and rules in pixels).
//! Nothing here touches a canvas; platform backends draw the display list.

pub mod ast;
pub mod display;
pub mod error;
pub mod font;
pub mod layout;
pub mod lexer;
pub mod parser;
pub mod symbols;

pub use ast::Node;
pub use display::{Color, DisplayList, Item};
pub use error::{Error, Result};
pub use font::MathFont;
pub use layout::{Layouter, RenderOptions};
pub use parser::parse;

/// Parses and lays out a formula in one call.
pub fn render(font: &MathFont<'_>, tex: &str, opts: &RenderOptions) -> Result<DisplayList> {
    let nodes = parse(tex)?;
    Ok(Layouter::new(font, opts).layout(&nodes, opts.display_mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");

    fn font() -> MathFont<'static> {
        MathFont::from_bytes(FONT).unwrap()
    }

    fn glyphs(dl: &DisplayList) -> Vec<(f32, f32, f32)> {
        dl.items
            .iter()
            .filter_map(|i| match i {
                Item::Glyph { x, y, size, .. } => Some((*x, *y, *size)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn renders_simple_expression() {
        let f = font();
        let dl = render(&f, "x+y=z", &RenderOptions::default()).unwrap();
        assert_eq!(glyphs(&dl).len(), 5);
        assert!(dl.width > 0.0 && dl.ascent > 0.0);
        // Everything sits on one baseline.
        let ys: Vec<f32> = glyphs(&dl).iter().map(|g| g.1).collect();
        assert!(ys.iter().all(|y| (y - ys[0]).abs() < 1e-3));
    }

    #[test]
    fn binary_spacing_is_wider_than_ordinary() {
        let f = font();
        let opts = RenderOptions::default();
        let tight = render(&f, "ab", &opts).unwrap();
        let spaced = render(&f, "a+b", &opts).unwrap();
        let plus = render(&f, "+", &opts).unwrap();
        assert!(spaced.width > tight.width + plus.width + 0.3 * opts.font_size);
        // Unary minus gets no binary spacing.
        let unary = render(&f, "-a", &opts).unwrap();
        let minus = render(&f, "-", &opts).unwrap();
        let a = render(&f, "a", &opts).unwrap();
        assert!((unary.width - (minus.width + a.width)).abs() < 0.5);
    }

    #[test]
    fn superscript_is_smaller_and_raised() {
        let f = font();
        let dl = render(&f, "x^2", &RenderOptions::default()).unwrap();
        let g = glyphs(&dl);
        assert_eq!(g.len(), 2);
        assert!(g[1].2 < g[0].2, "script must be smaller");
        assert!(g[1].1 < g[0].1, "superscript baseline must be above base baseline");
        let dl = render(&f, "x_2", &RenderOptions::default()).unwrap();
        let g = glyphs(&dl);
        assert!(g[1].1 > g[0].1, "subscript baseline must be below");
    }

    #[test]
    fn fraction_has_rule_between_num_and_den() {
        let f = font();
        let dl = render(&f, r"\frac{a}{b}", &RenderOptions::default()).unwrap();
        let g = glyphs(&dl);
        let rule = dl.items.iter().find_map(|i| match i {
            Item::Rule { y, height, .. } => Some((*y, *height)),
            _ => None,
        });
        let (ry, rh) = rule.expect("fraction rule");
        assert!(rh > 0.5);
        assert!(g[0].1 < ry, "numerator above rule");
        assert!(g[1].1 > ry + rh, "denominator below rule");
    }

    #[test]
    fn sqrt_grows_with_content() {
        let f = font();
        let opts = RenderOptions::default();
        let small = render(&f, r"\sqrt{x}", &opts).unwrap();
        let tall = render(&f, r"\sqrt{\frac{a}{b}}", &opts).unwrap();
        assert!(tall.height() > small.height() * 1.5);
    }

    #[test]
    fn left_right_delimiters_scale() {
        let f = font();
        let opts = RenderOptions::default();
        let plain = render(&f, r"(x)", &opts).unwrap();
        let tall = render(&f, r"\left(\frac{a}{b}\right)", &opts).unwrap();
        let paren_plain = glyphs(&plain)[0].2;
        let paren_tall = glyphs(&tall)[0];
        // The tall paren is either a bigger variant glyph (different id) or an assembly.
        assert!(tall.height() > plain.height());
        assert_eq!(paren_tall.2, paren_plain, "variants keep the em size, they change glyph id");
    }

    #[test]
    fn quadratic_formula_renders_without_error() {
        let f = font();
        let dl = render(&f, r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", &RenderOptions::default()).unwrap();
        assert!(glyphs(&dl).len() >= 12);
    }

    #[test]
    fn sum_with_limits_in_display_only() {
        let f = font();
        let d = render(
            &f,
            r"\sum_{i=1}^{n} i",
            &RenderOptions {
                display_mode: true,
                ..Default::default()
            },
        )
        .unwrap();
        let t = render(
            &f,
            r"\sum_{i=1}^{n} i",
            &RenderOptions {
                display_mode: false,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(d.height() > t.height(), "display limits stack vertically");
        assert!(d.width < t.width, "text style scripts sit to the side");
    }

    #[test]
    fn matrix_and_cases() {
        let f = font();
        let opts = RenderOptions::default();
        let m = render(&f, r"\begin{pmatrix} 1 & 0 \\ 0 & 1 \end{pmatrix}", &opts).unwrap();
        assert!(glyphs(&m).len() >= 6);
        let c = render(&f, r"f(x) = \begin{cases} 1 & x > 0 \\ 0 & \text{otherwise} \end{cases}", &opts).unwrap();
        assert!(c.height() > 2.0 * opts.font_size);
    }

    #[test]
    fn parse_error_is_reported() {
        let f = font();
        let e = render(&f, r"\frac{a", &RenderOptions::default()).unwrap_err();
        assert!(matches!(e, Error::Parse { .. }));
    }
}
