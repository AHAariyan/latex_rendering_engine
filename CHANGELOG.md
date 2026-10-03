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

### Accessibility

- Speech in English, Spanish, French, German, Portuguese, Bengali and
  Hindi, following the device or page language, at three verbosity levels.
- A speech tree that VoiceOver and TalkBack users step through part by
  part, each part outlined where it is drawn.
- Nemeth braille.
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

- Speech in languages other than English has not yet been reviewed by
  native speakers.
- Nemeth braille covers the code's core rules.
- No editing model (a math input field) yet.
- siunitx options (`[per-mode=symbol]` and the like) are read but not
  applied: output follows siunitx's defaults. Unit names are spoken in
  English only; other languages read the symbols.
- physics' `\sin(x)`-style automatic brackets are not applied, since
  documents without the package write `\sin(x)` too.
