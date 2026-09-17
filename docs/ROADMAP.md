# Roadmap

## Phase 0: core that renders real formulas (done, this commit)

Parser, OpenType MATH layout, display list, headless raster/SVG backend,
CLI, golden-image tests, CI.

## Phase 1: correctness and coverage

- [x] Side-by-side comparison harness against LuaLaTeX (`tools/texcompare`).
- [x] Math kerning from the MATH table (`MathKernInfo`), tested with STIX Two.
- [x] `\middle`, `\cancel`, `\boxed`, `\underbrace` `\overbrace`, `\xrightarrow`,
      `\substack`, `\pmod`, `\bmod`, `\choose`, `\atop`, `\genfrac`.
- [x] Colors: `\color`, `\textcolor`. `\colorbox` still open.
- [x] User macros: `\newcommand`, `\def`, `\renewcommand`, `\providecommand`,
      and a host macro table (`RenderOptions::macros`).
- [x] Unicode input: `α`, `≤`, `∑` typed directly.
- [x] `\text{}` shaping: built-in GPOS `kern` + GSUB `liga` over ttf-parser
      (rustybuzz was tried and dropped: +560 KB per binary for Unicode tables).
- [x] `ssty` script-style alternates, LaTeX array struts and interline glue,
      amsmath `cases`/`smallmatrix`/`\substack` dimensions, TeX rule 15e fraction
      delimiters, `\big` sizes via rule 19, absolute `\nulldelimiterspace`.
- [x] Array extras: `|` column rules, `\hline`, row spacing `\\[2pt]`.
- [ ] `\hdashline`, `\colorbox`, `\fcolorbox`, `\rule`, `\raisebox`, `\tag` display.
- [ ] Wide accents `\overrightarrow` `\overleftarrow` using horizontal assemblies.
- [x] STIX Two Math and Libertinus Math in the golden corpus.
- [x] Font subsetting (`scripts/subset-font.sh` + `tools/subset/repair_math.py`,
      equivalence test over corpus geometry and every MATH record). Bindings ship
      the 457 KB subset; the Android library is 1.05 MB per ABI.
- [x] Fuzz test for panics and stack depth; parser nesting limit.
- [x] Fix drift found by the TeX comparison harness. Remaining: `\left`/`\right`
      around nested delimiters is ~5% taller than LuaLaTeX; `\mathsf`/`\mathtt`
      fall back differently in unicode-math.

## Phase 2: platform bindings

- [x] C ABI (`mathffi` crate): `render(tex, opts) -> DisplayList` with a flat,
      FFI-friendly encoding; glyph outlines on demand for backends without the font.
- [x] Android: JNI bridge (`crates/mathjni`) + `MathEngine`/`MathView`/`MathText` in
      `platforms/android/mathview`, demo app. AAR publishing to Maven still open.
- [~] iOS: Swift package over the C ABI with Core Graphics drawing, UIKit and SwiftUI views
      (`platforms/ios/MathCore`). Written on Linux; needs a first Xcode build.
- [~] Flutter: `mathcore_dart` (pure `dart:ffi`, tested on host) + `mathcore_flutter`
      widget plugin (written; needs a first build with the Flutter SDK). pub.dev publishing open.
- [ ] React Native: Nitro/JSI module reusing the native renderers.
- [x] Web: wasm-bindgen + SVG/canvas renderer (`crates/mathwasm`, `platforms/web`), Node smoke test. npm publishing open.
- [ ] Demo apps and benchmarks (target: < 1 ms layout for a typical equation on a mid-range phone).

## Phase 3: product features

- [ ] Display-math line breaking for long equations on narrow screens.
- [ ] Accessibility: MathML and spoken-text output from the AST.
- [ ] Editing model: cursor, selection and incremental relayout for a math input control.
- [ ] Chemistry (`mhchem`), physics package macros, `siunitx` subset.
- [ ] Multiple fonts (STIX Two, Fira Math) and font fallback for missing glyphs.
- [ ] Layout caching keyed by (source, font, size, style).
