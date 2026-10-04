# Changelog

## 0.1.0-beta.1

The first public release: a native TeX math engine with SDKs for every
major platform. No WebView, no JavaScript in the layout path.

### Rendering

- TeX math laid out by the TeXbook's rules from the font's OpenType MATH
  table, as XeTeX, LuaTeX and Microsoft Word do.
- 98.2% of 103,559 real arXiv formulas render (KaTeX: 94.1%); 94% of
  KaTeX's command set.
- Text mode with `$...$`, font switches, accents and sizes; `\tag`
  equation numbers; chemistry with `\ce` and `\pu` (mhchem); commutative
  diagrams (`CD`); AsciiMath input.
- The physics package (derivatives, brackets, Dirac notation, vector
  operators, matrices) and siunitx (numbers, units, quantities, angles,
  ranges, lists), each matching LaTeX with that package loaded on all 49
  test formulas.
- Line breaking to a width, hit testing and highlighting, a layout cache,
  and budgets for formulas from untrusted users.
- Checked against LuaLaTeX: common formulas within 1–2%, almost all
  within 5%.

### Editing

- A math input field on every platform, backed by one editor model in the
  core: structural typing (`/`, `^`, `_`, brackets, `\commands`, automatic
  `sqrt`, `pi`, `sin`...), arrow navigation through structures, selection,
  undo and redo, copy and paste as TeX, a caret and selection placed from
  the engine's own layout, and screen-reader announcements of the cursor's
  place in 35 languages.

### Accessibility

- Speech in 35 languages, from Arabic and Bengali to Chinese, Japanese,
  Swahili and Urdu, following the device or page language, at three
  verbosity levels. Each formula structure is a template a language orders
  its own way; every table is complete (a test enforces it) and was
  back-translated against the English to catch reversed meanings.
- A speech tree that VoiceOver and TalkBack users step through part by
  part, each part outlined where it is drawn.
- Nemeth braille.
- Text in any writing system inside formulas: bidi, full shaping, and
  fonts found automatically on every platform.
- Numbers read as numbers ("9.81", not "9 . 8 1"), `f'` as "f prime",
  `90^\circ` as "90 degrees", siunitx units by name.
- MathML output.

### SDKs

| Platform | Package |
| --- | --- |
| Web | `mathcore` on npm, with the `<math-tex>` element |
| Android | `dev.mathcore:mathcore-android` (Compose and Views) |
| iOS, macOS | `MathCore` Swift package, CocoaPods |
| Flutter | `mathcore_flutter`, `mathcore_dart` on pub.dev |
| React Native | `react-native-mathcore` (New Architecture) |
| C and anything with a C FFI | `mathcore-c` archives |

### Known limitations

- Speech in languages other than English has been machine-written and
  cross-checked, not yet reviewed by native speakers. Languages that put
  the verb last (Hindi, Bengali, Tamil, Japanese, Korean...) read
  relations in a fixed infix form, which is correct but terse.
- Hebrew word spacing differs slightly from LuaLaTeX's (the glyphs match).
- Nemeth braille covers the code's core rules.
- The math field edits fractions, roots, scripts and bracket pairs;
  matrices and `\text{}` are kept as single units, not edited inside. No
  on-screen math keyboard on Android, Flutter or the web yet (iOS has a key
  bar); the field has no native macOS (AppKit) view.
- siunitx options (`[per-mode=symbol]` and the like) are read but not
  applied: output follows siunitx's defaults. Unit names are spoken in
  English only; other languages read the symbols.
- physics' `\sin(x)`-style automatic brackets are not applied, since
  documents without the package write `\sin(x)` too.
