# Roadmap

## Phase 0: core that renders real formulas (done, this commit)

Parser, OpenType MATH layout, display list, headless raster/SVG backend,
CLI, golden-image tests, CI.

## Phase 1: correctness and coverage

- [ ] Side-by-side comparison harness: render the corpus with real LaTeX
      (`latex` + `dvisvgm`) and overlay against our output to find drift.
      Blocked locally: no TeX installation on the dev machine.
- [x] Math kerning from the MATH table (`MathKernInfo`). Implemented; Latin
      Modern Math ships no kern data, so it only shows with STIX Two / Libertinus.
- [x] `\middle`, `\cancel`, `\boxed`, `\underbrace` `\overbrace`, `\xrightarrow`,
      `\substack`, `\pmod`, `\bmod`, `\choose`, `\atop`, `\genfrac`.
- [x] Colors: `\color`, `\textcolor`. `\colorbox` still open.
- [x] User macros: `\newcommand`, `\def`, `\renewcommand`, `\providecommand`,
      and a host macro table (`RenderOptions::macros`).
- [x] Unicode input: `α`, `≤`, `∑` typed directly.
- [ ] Better `\text{}`: real shaping via `rustybuzz` for kerning/ligatures and
      non-Latin scripts, or delegate text runs to the platform.
- [x] Array extras: `|` column rules, `\hline`, row spacing `\\[2pt]`.
- [ ] `\hdashline`, `\colorbox`, `\fcolorbox`, `\rule`, `\raisebox`, `\tag` display.
- [ ] Wide accents `\overrightarrow` `\overleftarrow` using horizontal assemblies.
- [ ] Second font (STIX Two Math) in the golden corpus to catch font-specific assumptions.
- [ ] Font subsetting for the bundled font and a size budget (< 1 MB core+font).

## Phase 2: platform bindings

- [x] C ABI (`mathffi` crate): `render(tex, opts) -> DisplayList` with a flat,
      FFI-friendly encoding; glyph outlines on demand for backends without the font.
- [x] Android: JNI bridge (`crates/mathjni`) + `MathEngine`/`MathView`/`MathText` in
      `platforms/android/mathview`, demo app. AAR publishing to Maven still open.
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
