//! Prints every code point the engine can reach, one `U+XXXX` per line, for
//! font subsetting (see scripts/subset-font.sh).
use mathcore::ast::Variant;
use mathcore::symbols::{styled_char, ACCENTS, BIG_OPS, SYMBOLS};
use std::collections::BTreeSet;

fn main() {
    let mut set = BTreeSet::new();
    // Printable ASCII and Latin-1 punctuation, plus TeX's replacements.
    for c in (0x20u32..0x7F).chain(0xA0..0x100) {
        set.insert(char::from_u32(c).unwrap());
    }
    for c in [
        '−', '∗', '′', '″', '‴', '⁗', '√', '⏞', '⏟', '‖', '⟨', '⟩', '⌊', '⌋', '⌈', '⌉', '⟦', '⟧', '⟮', '⟯', '⎰', '⎱', '‘', '’', '“', '”',
    ] {
        set.insert(c);
    }
    for (_, c, _) in SYMBOLS {
        set.insert(*c);
    }
    for (_, c, _) in BIG_OPS {
        set.insert(*c);
    }
    for (_, c, _) in ACCENTS {
        set.insert(*c);
    }
    // Greek and every math alphabet reachable through \mathbf & co.
    let bases: Vec<char> = ('A'..='Z')
        .chain('a'..='z')
        .chain('0'..='9')
        .chain('Α'..='Ω')
        .chain('α'..='ω')
        .chain(['ϵ', 'ϑ', 'ϰ', 'ϕ', 'ϱ', 'ϖ', '∂', '∇', 'ϴ'])
        .collect();
    for v in [
        Variant::Normal,
        Variant::Roman,
        Variant::Bold,
        Variant::Italic,
        Variant::BoldItalic,
        Variant::Script,
        Variant::Fraktur,
        Variant::DoubleStruck,
        Variant::SansSerif,
        Variant::Monospace,
    ] {
        for &b in &bases {
            set.insert(styled_char(b, v));
        }
    }
    // Negations produced by \not.
    for c in [
        '≠', '∉', '≮', '≯', '≢', '⊄', '⊅', '⊈', '⊉', '≁', '≉', '≰', '≱', '∄', '≄', '≇', '∤', '∦', '↛', '⇏', '∌',
    ] {
        set.insert(c);
    }
    for c in set {
        println!("U+{:04X}", c as u32);
    }
}
