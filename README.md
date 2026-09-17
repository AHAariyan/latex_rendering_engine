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
  arrows, delimiters, `\frac` `\dfrac` `\tfrac` `\binom`, `\sqrt[n]`, scripts and
  primes, `\left` `\right`, `\big` sizes, accents including stretchy `\widehat`,
  `\overline` `\underline`, `\mathbf` and the other math alphabets, `\text`,
  `\operatorname`, spacing commands, style commands, `\phantom`, `\not`,
  `\overset` `\underset`, and the `matrix`, `pmatrix`, `bmatrix`, `Bmatrix`,
  `vmatrix`, `Vmatrix`, `smallmatrix`, `cases`, `array`, `aligned`, `gathered`
  environments.
- Layout: atom spacing with Bin/Ord rewriting, scripts, limits, fractions and
  stacks, radicals with index, extensible delimiters via size variants and
  glyph assembly, large operators, accents with attachment points, arrays.
- Backends: headless raster (PNG) and SVG in `mathraster`, used for tests and
  by the `mathcli` tool.

Not yet: user macros, line breaking, colors, `\middle`, `\cancel`, `mhchem`,
accessibility output, and the mobile bindings. See `docs/ROADMAP.md`.

## Try it

```sh
cargo run --release -p mathcli -- 'x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}' -o quad.png
cargo run --release -p mathcli -- --svg '\sum_{n=1}^\infty \frac{1}{n^2}' -o basel.svg
cargo run --release -p mathcli -- --inline --size 20 'e^{i\pi}+1=0' -o euler.png
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
