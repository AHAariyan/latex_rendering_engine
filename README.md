# latex_rendering_engine

A native, cross-platform TeX math typesetting engine. No WebView, no JavaScript,
no platform text stack in the layout path.

The core is a single Rust library that turns TeX math into a **display list**:
glyph ids with positions and sizes, plus filled rectangles. A platform backend
draws that list with its own canvas in a few hundred lines. Layout follows the
TeXbook Appendix G rules with every dimension read from the font's OpenType
`MATH` table, which is the same approach as XeTeX, LuaTeX and Microsoft Word.

## Status

Early, but rendering real formulas end to end. See `tests/golden/` for the
current output of the 30-formula regression corpus.

Working today:

- Parser covering KaTeX's command set: 585 symbols including the AMS
  relations, operators and arrows, every math alphabet, `\frac` and its family,
  `\genfrac` and the infix `\over` `\above` `\choose` `\atop`, `\sqrt[n]`,
  scripts and primes, `\left` `\middle` `\right`, `\big` sizes, accents
  above and below including the stretchy `\widehat` `\overleftrightarrow`
  `\underrightarrow` `\overgroup` family, `\underbrace` `\overbrace`, the
  `\xrightarrow` arrows, `\boxed` `\colorbox` `\fcolorbox` `\cancel`
  `\sout`, `\rule` `\raisebox` `\llap` `\rlap` `\clap` `\vcenter`
  `\mathchoice`, `\verb`, `\text` and the font switches, `\operatorname`,
  the class overrides, spacing commands with real dimensions, `\phantom` and
  `\smash`, `\not`, `\overset` `\underset`, `\substack`, `\pmod`,
  `\color` `\textcolor`, function names from several languages, and the
  `matrix` family, `cases`, `array` with rules, `aligned`, `gathered` and
  `subarray` environments.

- Macros: `\newcommand`, `\renewcommand`, `\providecommand`, `\def` in the
  source, plus host-supplied definitions through `RenderOptions::macros`.
- Unicode input: `α ≤ ∑` typed directly.
- **Hit testing and selection**: with `hit_testing` on, the layout records the
  byte range of source behind every piece of the drawing. A tap maps back to a
  sub-expression, regions nest so a host can offer the symbol or the whole
  fraction, and `highlight` returns the rectangles covering a source range.
  Slicing the source gives copy-as-LaTeX for free.
- **Accessibility**: Presentation MathML and a spoken sentence, written from
  the same tree the engine draws from, so `x^2 + y^2 = z^2` is announced as
  "x squared plus y squared equals z squared". The Compose, SwiftUI, Flutter
  and Android View widgets carry it automatically, so TalkBack and VoiceOver
  read the formula instead of skipping it.
- **Line breaking**: a formula too wide for the space available is broken into
  lines before relations and binary operators, the way an author breaks a long
  equation by hand, with the split chosen by a Knuth-Plass style dynamic
  program so the lines come out even. TeX does not do this at all, KaTeX does
  not try, and MathJax only breaks at top-level relations. On a phone it is the
  difference between a readable equation and a horizontal scrollbar.
- Layout: atom spacing with Bin/Ord rewriting, scripts with MATH-table
  kerning, limits, fractions and stacks, radicals with index, extensible
  delimiters via size variants and glyph assembly, large operators, accents
  with attachment points, arrays with LaTeX's exact struts and interline glue,
  `ssty` script-style glyph alternates, and a built-in shaper (GPOS `kern`,
  GSUB `liga`) for `\text{}` runs. No shaping library dependency.
- Backends: headless raster (PNG) and SVG in `mathraster`, used for tests and
  by the `mathcli` tool.
- C ABI in `mathffi` (`include/mathcore.h`): engine, render to a flat item
  array, glyph outlines as a command stream. This is what the Kotlin, Swift,
  Dart and JS bindings will wrap.

- Platforms, all over the same engine and display list:

  | Platform | Package | Status |
  | --- | --- | --- |
  | Android | `platforms/android/mathview` (Compose `MathText`, `MathView`, `MathEngine`) | Built and verified on an emulator |
  | Web | `crates/mathwasm` + `platforms/web` (SVG or canvas) | Built and tested in Node |
  | Dart | `platforms/dart/mathcore_dart` (pure `dart:ffi`) | Tested on the host |
  | Flutter | `platforms/flutter/mathcore_flutter` (`MathText` widget) | Written, not yet compiled with the Flutter SDK |
  | iOS | `platforms/ios/MathCore` (SwiftUI `MathText`, `MathView`) | Written, not yet compiled with Xcode |

- **Prose in your own font**: `\text{}` can be set in a font you supply while
  the maths stays in the math font, which is what makes a formula look like it
  belongs in the app. Text is shaped by whichever font sets it, so it gets that
  font's kerning and ligatures, and a right-to-left run such as Hebrew comes out
  in the right visual order. Arabic and the Indic scripts need joining forms and
  reordering: turn on the `complex-text` feature for those, which brings in a
  full shaper at a cost of about 600 KB.
- **Font fallback**: a font can carry a chain, and a character the primary
  lacks comes from the next one. Latin Modern Math predates 27 of the AMS
  symbols, so every binding ships a 5 KB slice of STIX Two alongside it and the
  whole table renders out of the box. Layout constants always come from the
  primary, so adding a fallback cannot change a formula that did not need it.

Not yet: `mhchem`, React Native, an editing model. See `docs/ROADMAP.md`.

## Try it

```sh
cargo run --release -p mathcli -- 'x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}' -o quad.png
cargo run --release -p mathcli -- --svg '\sum_{n=1}^\infty \frac{1}{n^2}' -o basel.svg
cargo run --release -p mathcli -- --inline --size 20 'e^{i\pi}+1=0' -o euler.png
cargo run --release -p mathcli -- --macro '\R=\mathbb{R}' 'f: \R \to \R' -o f.png
cargo run --release -p mathcli -- --width 360 'f(x) = a_0 + a_1 x + a_2 x^2 + a_3 x^3 + a_4 x^4' -o wrapped.png
```

## Use the library

```rust
let font = mathcore::MathFont::from_bytes(include_bytes!("latinmodern-math.otf"))?;
let opts = mathcore::RenderOptions { font_size: 32.0, display_mode: true, ..Default::default() };
let list = mathcore::render(&font, r"\frac{a}{b}", &opts)?;
// ... or fit it to a width:
let opts = mathcore::RenderOptions { line_break: Some(mathcore::LineBreak::new(360.0)), ..opts };
// Tap support:
let opts = mathcore::RenderOptions { hit_testing: true, ..opts };
// let region = list.hit_nearest(x, y);   // -> byte range of the source

// For a screen reader, with no font needed:
let spoken = mathcore::render_speech(r"\frac{a}{b}", &Default::default())?;
let markup = mathcore::render_mathml(r"\frac{a}{b}", true, &Default::default())?;
for item in &list.items {
    match item {
        mathcore::Item::Glyph { id, x, y, size, .. } => { /* draw glyph `id` at (x, y) with em size `size` */ }
        mathcore::Item::Rule { x, y, width, height, .. } => { /* fill rect */ }
    }
}
```

## Layout of the repository

| Path | What |
| --- | --- |
| `crates/mathcore` | Parser, font access, layout engine, display list. No I/O, no rendering. |
| `crates/mathraster` | tiny-skia raster backend and SVG writer. Reference implementation for platform backends. |
| `crates/mathcli` | Command line renderer. |
| `crates/mathffi` | C ABI (`cdylib` + `staticlib`) and `include/mathcore.h`. |
| `crates/mathjni` | JNI bridge for Android (`libmathcore_android.so`). |
| `crates/mathwasm` | wasm-bindgen binding for the web. |
| `platforms/web` | Canvas renderer, demo page, Node smoke test. |
| `platforms/dart`, `platforms/flutter` | Pure Dart FFI package and the Flutter widget plugin. |
| `platforms/ios` | Swift package with UIKit and SwiftUI views. |
| `scripts/build-android-ffi.sh`, `scripts/build-ios.sh` | Cross-compiles the C ABI for Flutter on Android and for iOS. |
| `platforms/android` | `mathview` Android library (Kotlin: `MathEngine`, `MathView`, Compose `MathText`) and demo app. |
| `scripts/build-android.sh` | Cross-compiles the JNI library for arm64, armv7 and x86_64. |
| `assets/fonts` | Latin Modern Math (GUST Font License, full + subset), the 5 KB STIX Two fallback slice, and STIX Two and Libertinus in full for tests (OFL). |

| `tools/texcompare` | Side-by-side comparison against LuaLaTeX. |
| `tools/subset` | MATH-table repair pass for subset fonts. |
| `tests/golden` | Golden images for the regression corpus. |
| `docs/` | Architecture and roadmap. |

## Quality gates

- **Golden images** for 58 formulas in three fonts (Latin Modern Math, STIX Two
  Math, Libertinus Math): `tests/golden/`.
- **Fuzzing**: `crates/mathcore/tests/robustness.rs` throws 20,000 random
  token soups and a set of pathological inputs at the engine on a small-stack
  thread. The engine never panics; nesting deeper than 64 levels is a parse
  error, and a release build handles that within 256 KB of stack.
- **Font subsetting**: the shipped `latinmodern-math-subset.otf` (457 KB, down
  from 734 KB) is proven identical to the full font by
  `crates/mathraster/tests/subset.rs`, both in corpus geometry and in every
  MATH record the engine reads. Regenerate with `scripts/subset-font.sh`,
  which runs `pyftsubset` and then `tools/subset/repair_math.py` to restore
  the italic corrections and accent attachment points fontTools drops for
  glyphs reachable only through GSUB.
- **Comparison against real TeX**: `tools/texcompare/compare.py` renders the
  corpus with LuaLaTeX + unicode-math using the same font at TeX's 10 pt and
  stacks the pairs into a contact sheet with size ratios; `--tex 'formula'`
  compares ad-hoc input. Needs `lualatex` and `pdftoppm`. As of this commit
  every corpus formula is within 5% of LuaLaTeX in both dimensions except the
  display-style `\binom` family, where LuaLaTeX picks a larger delimiter than
  TeX's own rule (and KaTeX/pdfLaTeX) calls for.
- **Cross-binding parity**: `scripts/parity.sh` has the WebAssembly and Dart
  bindings lay out the whole corpus and compares every glyph position against
  the Rust core, so the promise that a formula looks the same on every platform
  is checked rather than asserted. 60 formulas, 1060 items.
- **Limits for untrusted input**: `RenderOptions::budget` caps expanded source,
  parsed nodes and drawable items. A formula from a stranger that would cost
  real memory is an error, not an out-of-memory kill. Defaults are far above
  anything a person writes; `Budget::unlimited()` opts out.
- **Benchmarks**: `cargo bench -p mathcore`.

## Development

```sh
cargo test --workspace                                   # unit + golden tests
UPDATE_GOLDEN=1 cargo test -p mathraster --test golden   # regenerate goldens after an intended change
cargo clippy --workspace --all-targets -- -D warnings
```

Golden mismatches write the new output to `tests/golden/actual/` for comparison.

## License

MIT. The bundled Latin Modern Math font is under the GUST Font License, see
`assets/fonts/GUST-FONT-LICENSE.txt`.
