//! Prints the code points the primary bundled font is missing, for
//! `scripts/build-fallback.sh` to pull from a font that has them.
use mathcore::symbols::{ACCENTS, BIG_OPS, SYMBOLS};

fn main() {
    let font = mathcore::MathFont::from_bytes(include_bytes!("../../../assets/fonts/latinmodern-math-subset.otf")).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for c in SYMBOLS
        .iter()
        .map(|(_, c, _)| *c)
        .chain(BIG_OPS.iter().map(|(_, c, _)| *c))
        .chain(ACCENTS.iter().map(|(_, c, ..)| *c))
    {
        if font.glyph_index(c).is_none() {
            seen.insert(c as u32);
        }
    }
    for c in seen {
        println!("U+{c:04X}");
    }
}
