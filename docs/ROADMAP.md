# Roadmap

## Phase 0: core that renders real formulas (done, this commit)

Parser, OpenType MATH layout, display list, headless raster/SVG backend,
CLI, golden-image tests, CI.

## Phase 1: correctness against TeX

- [ ] Side-by-side comparison harness: render the corpus with real LaTeX
      (`latex` + `dvisvgm`) and overlay against our output to find drift.
- [ ] Math kerning from the MATH table (`MathKernInfo`) for tighter scripts on
      letters like `f`, `V`, `T`.
- [ ] `\middle`, `\cancel`, `\boxed`, `\underbrace` `\overbrace`, `\xrightarrow`,
      `\substack`, `\pmod`, `\bmod`, `\choose`, `\atop`, `\genfrac`.
- [ ] Colors: `\color`, `\textcolor`, `\colorbox`, per-item color already in the display list.
- [ ] User macros: `\newcommand`, `\def`, `\renewcommand` with arguments; a
      macro table passed in from the host (KaTeX `macros` option equivalent).
- [ ] Unicode input: allow `α`, `≤`, `∞` typed directly.
- [ ] Better `\text{}`: real shaping via `rustybuzz` for kerning/ligatures and
      non-Latin scripts, or delegate text runs to the platform.
- [ ] Array extras: `|` column rules, `\hline`, `\hdashline`, row spacing `\\[2pt]`.
- [ ] Wide accents `\overrightarrow` `\overleftarrow` using horizontal assemblies.
- [ ] Font subsetting for the bundled font and a size budget (< 1 MB core+font).

## Phase 2: platform bindings

- [ ] C ABI (`mathffi` crate): `render(tex, opts) -> DisplayList` with a flat,
      FFI-friendly encoding; glyph outlines on demand for backends without the font.
- [ ] Android: UniFFI Kotlin bindings + `Canvas`/Compose renderer + AAR on Maven.
- [ ] iOS: UniFFI Swift bindings + Core Text renderer + SwiftPM package.
- [ ] Flutter: `dart:ffi` + `CustomPainter` renderer + pub.dev package.
- [ ] React Native: Nitro/JSI module reusing the native renderers.
- [ ] Web: wasm-bindgen + canvas/SVG renderer + npm package (fallback and parity testing).
- [ ] Demo apps and benchmarks (target: < 1 ms layout for a typical equation on a mid-range phone).

## Phase 3: product features

- [ ] Display-math line breaking for long equations on narrow screens.
- [ ] Accessibility: MathML and spoken-text output from the AST.
- [ ] Editing model: cursor, selection and incremental relayout for a math input control.
- [ ] Chemistry (`mhchem`), physics package macros, `siunitx` subset.
- [ ] Multiple fonts (STIX Two, Fira Math) and font fallback for missing glyphs.
- [ ] Layout caching keyed by (source, font, size, style).
