//! Golden-image regression tests.
//!
//! Every formula in `CORPUS` is rendered headlessly and compared pixel-wise
//! with `tests/golden/<name>.png` at the repository root. Run with
//! `UPDATE_GOLDEN=1 cargo test -p mathraster --test golden` to regenerate
//! after an intentional layout change, then inspect the diff in git before
//! committing.

use mathcore::{MathFont, RenderOptions};
use mathraster::{rasterize, RasterOptions};
use std::path::PathBuf;

const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");

/// (name, tex, display_mode)
const CORPUS: &[(&str, &str, bool)] = &[
    ("quadratic", r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", true),
    ("basel", r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}", true),
    ("gaussian", r"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}", true),
    ("euler", r"e^{i\pi} + 1 = 0", false),
    ("limit", r"\lim_{x \to 0} \frac{\sin x}{x} = 1", true),
    ("limit_inline", r"\lim_{x \to 0} \frac{\sin x}{x} = 1", false),
    ("binom", r"\binom{n}{k} = \frac{n!}{k!(n-k)!}", true),
    (
        "cases",
        r"f(x) = \begin{cases} x^2 & \text{if } x \ge 0 \\ -x & \text{otherwise} \end{cases}",
        true,
    ),
    ("pmatrix", r"A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}", true),
    (
        "bmatrix3",
        r"\begin{bmatrix} 1 & 0 & 0 \\ 0 & 1 & 0 \\ 0 & 0 & 1 \end{bmatrix}",
        true,
    ),
    ("aligned", r"\begin{aligned} a &= b + c \\ d + e &= f \end{aligned}", true),
    ("left_right", r"\left( \frac{a}{b} + \left[ \frac{c}{d} \right] \right)^2", true),
    ("nested_frac", r"\frac{1}{1 + \frac{1}{1 + \frac{1}{x}}}", true),
    ("sqrt_index", r"\sqrt[3]{x^3 + y^3} + \sqrt{\frac{a}{b}}", true),
    (
        "accents",
        r"\hat{x} \tilde{y} \bar{z} \vec{v} \dot{q} \ddot{q} \widehat{abc} \widetilde{xyz} \overline{AB} \underline{CD}",
        true,
    ),
    (
        "alphabets",
        r"\mathbf{Ab} \mathbb{RZ} \mathcal{FG} \mathfrak{gh} \mathsf{xy} \mathtt{01} \boldsymbol{\alpha\beta}",
        true,
    ),
    (
        "greek",
        r"\alpha\beta\gamma\delta\epsilon\varepsilon\zeta\eta\theta\vartheta\iota\kappa\lambda\mu\nu\xi\pi\rho\sigma\tau\upsilon\phi\varphi\chi\psi\omega \Gamma\Delta\Theta\Lambda\Xi\Pi\Sigma\Upsilon\Phi\Psi\Omega",
        true,
    ),
    (
        "relations",
        r"a \le b \ge c \ne d \approx e \equiv f \subset g \in h \to i \Rightarrow j \iff k",
        true,
    ),
    ("spacing", r"a+b-c\times d\cdot e = f, g; h \, i \; j \quad k \qquad l", true),
    (
        "scripts",
        r"x_i^2 + y^{a^{b^c}} + z_{i_{j_k}} + f'(x) + g''(x) + T_{\mathrm{max}}^{\mathrm{ref}}",
        true,
    ),
    (
        "big_ops_display",
        r"\sum_{i=1}^{n} \prod_{j=1}^{m} \int_0^1 \oint_C \bigcup_{k} \bigcap_{k} x",
        true,
    ),
    ("big_ops_inline", r"\sum_{i=1}^{n} \prod_{j=1}^{m} \int_0^1 \oint_C x", false),
    (
        "maxwell",
        r"\nabla \cdot \mathbf{E} = \frac{\rho}{\varepsilon_0}, \quad \nabla \times \mathbf{B} - \frac{1}{c^2}\frac{\partial \mathbf{E}}{\partial t} = \mu_0 \mathbf{J}",
        true,
    ),
    (
        "schrodinger",
        r"i\hbar \frac{\partial}{\partial t} \Psi(\mathbf{r},t) = \left[ -\frac{\hbar^2}{2m} \nabla^2 + V(\mathbf{r},t) \right] \Psi(\mathbf{r},t)",
        true,
    ),
    (
        "sized_delims",
        r"\bigl( \Bigl( \biggl( \Biggl( x \Biggr) \biggr) \Bigr) \bigr)",
        true,
    ),
    (
        "text_and_ops",
        r"\sin^2\theta + \cos^2\theta = 1, \quad \log_2 8 = 3, \quad \operatorname{argmax}_x f(x)",
        true,
    ),
    (
        "stack",
        r"\overset{\text{def}}{=} \quad \underset{x}{\max} \quad \stackrel{?}{=}",
        true,
    ),
    ("not_and_neg", r"a \not= b, \quad x \notin S, \quad \neg p \lor q", true),
    ("phantom", r"\frac{1}{\phantom{-}2} + \frac{1}{-2}", true),
    (
        "smallmatrix",
        r"\left( \begin{smallmatrix} a & b \\ c & d \end{smallmatrix} \right)",
        false,
    ),
];

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

#[test]
fn golden_images() {
    let font = MathFont::from_bytes(FONT).unwrap();
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let dir = golden_dir();
    let actual_dir = dir.join("actual");
    std::fs::create_dir_all(&actual_dir).unwrap();
    let mut failures = Vec::new();
    for (name, tex, display) in CORPUS {
        let opts = RenderOptions {
            font_size: 32.0,
            display_mode: *display,
            ..Default::default()
        };
        let dl = match mathcore::render(&font, tex, &opts) {
            Ok(dl) => dl,
            Err(e) => {
                failures.push(format!("{name}: render error: {e}"));
                continue;
            }
        };
        let px = rasterize(
            &font,
            &dl,
            &RasterOptions {
                scale: 1.0,
                padding: 4.0,
                background: Some(mathcore::Color(255, 255, 255, 255)),
            },
        )
        .unwrap();
        let path = dir.join(format!("{name}.png"));
        let png = px.encode_png().unwrap();
        if update || !path.exists() {
            std::fs::write(&path, &png).unwrap();
            continue;
        }
        let expected = mathraster::tiny_skia::Pixmap::decode_png(&std::fs::read(&path).unwrap()).unwrap();
        let same_size = expected.width() == px.width() && expected.height() == px.height();
        let mismatch = if same_size {
            expected
                .data()
                .chunks(4)
                .zip(px.data().chunks(4))
                .filter(|(a, b)| a[0].abs_diff(b[0]) > 8)
                .count()
        } else {
            usize::MAX
        };
        let total = (px.width() * px.height()) as usize;
        if !same_size || mismatch as f64 / total as f64 > 0.002 {
            std::fs::write(actual_dir.join(format!("{name}.png")), &png).unwrap();
            failures.push(format!(
                "{name}: {} (expected {}x{}, got {}x{}, {mismatch} pixels differ); actual written to tests/golden/actual/",
                tex,
                expected.width(),
                expected.height(),
                px.width(),
                px.height()
            ));
        }
    }
    assert!(failures.is_empty(), "golden mismatches:\n{}", failures.join("\n"));
}
