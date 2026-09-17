//! Golden-image regression tests.
//!
//! Every formula in `CORPUS` is rendered headlessly with each font and
//! compared pixel-wise with `tests/golden/<name>.png` at the repository root.
//! Run with `UPDATE_GOLDEN=1 cargo test -p mathraster --test golden` to
//! regenerate after an intentional layout change, then inspect the diff in git
//! before committing.

use mathcore::{LineBreak, MathFont, RenderOptions};
use mathraster::{rasterize, tiny_skia::Pixmap, RasterOptions};
use std::path::{Path, PathBuf};

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
use corpus::{CORPUS, LINEBREAK};

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden")
}

struct Session {
    dir: PathBuf,
    actual: PathBuf,
    update: bool,
    failures: Vec<String>,
}

impl Session {
    fn new() -> Session {
        let dir = golden_dir();
        let actual = dir.join("actual");
        std::fs::create_dir_all(&actual).unwrap();
        Session {
            dir,
            actual,
            update: std::env::var_os("UPDATE_GOLDEN").is_some(),
            failures: Vec::new(),
        }
    }

    fn render(&mut self, font: &MathFont<'_>, name: &str, tex: &str, opts: &RenderOptions) {
        let dl = match mathcore::render(font, tex, opts) {
            Ok(dl) => dl,
            Err(e) => return self.failures.push(format!("{name}: render error: {e}")),
        };
        let raster = RasterOptions {
            scale: 1.0,
            padding: 4.0,
            background: Some(mathcore::Color(255, 255, 255, 255)),
        };
        self.compare(name, tex, rasterize(font, &dl, &raster).unwrap());
    }

    fn compare(&mut self, name: &str, tex: &str, px: Pixmap) {
        let path: &Path = &self.dir.join(format!("{name}.png"));
        let png = px.encode_png().unwrap();
        if self.update || !path.exists() {
            std::fs::write(path, &png).unwrap();
            return;
        }
        let expected = Pixmap::decode_png(&std::fs::read(path).unwrap()).unwrap();
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
            std::fs::write(self.actual.join(format!("{name}.png")), &png).unwrap();
            self.failures.push(format!(
                "{name}: {tex} (expected {}x{}, got {}x{}, {mismatch} pixels differ); actual written to tests/golden/actual/",
                expected.width(),
                expected.height(),
                px.width(),
                px.height()
            ));
        }
    }
}

#[test]
fn golden_images() {
    let mut s = Session::new();
    for (suffix, bytes) in FONTS {
        // The default column is the chain every binding ships: the primary font
        // plus the small fallback that fills its gaps.
        let font = if suffix.is_empty() {
            mathcore::bundled::font().unwrap()
        } else {
            MathFont::from_bytes(bytes).unwrap()
        };
        for (base, tex, display) in CORPUS {
            let name = if suffix.is_empty() {
                base.to_string()
            } else {
                format!("{base}.{suffix}")
            };
            let opts = RenderOptions {
                font_size: 32.0,
                display_mode: *display,
                ..Default::default()
            };
            s.render(&font, &name, tex, &opts);
        }
    }
    // Line breaking, with the default font only.
    let font = mathcore::bundled::font().unwrap();
    for (base, tex, width) in LINEBREAK {
        let opts = RenderOptions {
            font_size: 32.0,
            line_break: Some(LineBreak::new(*width)),
            ..Default::default()
        };
        s.render(&font, &format!("lb_{base}"), tex, &opts);
    }
    assert!(s.failures.is_empty(), "golden mismatches:\n{}", s.failures.join("\n"));
}
