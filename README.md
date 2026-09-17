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

- Parser for the KaTeX-style command set: symbols, Greek, operators, relations,
  arrows, delimiters, `\frac` `\dfrac` `\tfrac` `\cfrac` `\binom` `\genfrac`
  and the infix `\over` `\choose` `\atop`, `\sqrt[n]`, scripts and primes,
  `\left` `\middle` `\right`, `\big` sizes, accents including stretchy
  `\widehat`, `\overline` `\underline`, `\underbrace` `\overbrace`,
  `\xrightarrow` and the other extensible arrows, `\boxed`, `\cancel`,
  `\mathbf` and the other math alphabets, `\text`, `\operatorname`,
  `\mathop` `\mathrel` and the other class overrides, spacing commands and
  `\hspace` `\kern` with dimensions, style commands, `\phantom` `\vphantom`
  `\hphantom` `\smash`, `\not`, `\overset` `\underset`, `\substack`,
  `\pmod` `\bmod`, `\color` `\textcolor`, and the `matrix` family,
  `cases`, `array` with `|` and `\hline`, `aligned`, `gathered` environments.
- Macros: `\newcommand`, `\renewcommand`, `\providecommand`, `\def` in the
  source, plus host-supplied definitions through `RenderOptions::macros`.
- Unicode input: `α ≤ ∑` typed directly.
- Layout: atom spacing with Bin/Ord rewriting, scripts with MATH-table
  kerning, limits, fractions and stacks, radicals with index, extensible
  delimiters via size variants and glyph assembly, large operators, accents
  with attachment points, arrays with rules.
- Backends: headless raster (PNG) and SVG in `mathraster`, used for tests and
  by the `mathcli` tool.
- C ABI in `mathffi` (`include/mathcore.h`): engine, render to a flat item
  array, glyph outlines as a command stream. This is what the Kotlin, Swift,
  Dart and JS bindings will wrap.

- Android: `platforms/android/mathview` with a Compose `MathText`, a classic
  `MathView`, and `MathEngine` for custom drawing. See `platforms/android/README.md`.

Not yet: line breaking, `mhchem`, accessibility output, real text shaping in
`\text{}`, and the iOS, Flutter, React Native and web bindings. See `docs/ROADMAP.md`.

## Try it

```sh
cargo run --release -p mathcli -- 'x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}' -o quad.png
cargo run --release -p mathcli -- --svg '\sum_{n=1}^\infty \frac{1}{n^2}' -o basel.svg
cargo run --release -p mathcli -- --inline --size 20 'e^{i\pi}+1=0' -o euler.png
cargo run --release -p mathcli -- --macro '\R=\mathbb{R}' 'f: \R \to \R' -o f.png
```

## Use the library

```rust
let font = mathcore::MathFont::from_bytes(include_bytes!("latinmodern-math.otf"))?;
let opts = mathcore::RenderOptions { font_size: 32.0, display_mode: true, ..Default::default() };
let list = mathcore::render(&font, r"\frac{a}{b}", &opts)?;
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
| `platforms/android` | `mathview` Android library (Kotlin: `MathEngine`, `MathView`, Compose `MathText`) and demo app. |
| `scripts/build-android.sh` | Cross-compiles the JNI library for arm64, armv7 and x86_64. |
| `assets/fonts` | Latin Modern Math (GUST Font License). |
| `tests/golden` | Golden images for the regression corpus. |
| `docs/` | Architecture and roadmap. |

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
