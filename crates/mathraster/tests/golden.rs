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

/// Fonts in the corpus. Every formula is rendered with each one so that
/// font-specific assumptions surface. File names are `<name>.png` for Latin
/// Modern and `<name>.<font>.png` for the others.
const FONTS: &[(&str, &[u8])] = &[
    ("", include_bytes!("../../../assets/fonts/latinmodern-math.otf")),
    ("stix", include_bytes!("../../../assets/fonts/STIXTwoMath-Regular.otf")),
    ("libertinus", include_bytes!("../../../assets/fonts/LibertinusMath-Regular.otf")),
];

#[path = "common/corpus.rs"]
mod corpus;
use corpus::CORPUS;

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

#[test]
fn golden_images() {
    let update = std::env::var_os("UPDATE_GOLDEN").is_some();
    let dir = golden_dir();
    let actual_dir = dir.join("actual");
    std::fs::create_dir_all(&actual_dir).unwrap();
    let mut failures = Vec::new();
    for (suffix, bytes) in FONTS {
        let font = MathFont::from_bytes(bytes).unwrap();
        for (base, tex, display) in CORPUS {
            let name = if suffix.is_empty() {
                base.to_string()
            } else {
                format!("{base}.{suffix}")
            };
            let name = name.as_str();
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
    }
    assert!(failures.is_empty(), "golden mismatches:\n{}", failures.join("\n"));
}
