# Architecture

## Goal

One layout core, many thin drawing backends. The core must be deterministic,
allocation-light, and free of any platform dependency so that identical input
produces identical output on Android, iOS, desktop, Flutter, React Native and
the web.

## Pipeline

```
TeX source ──lexer──▶ tokens ──parser──▶ Node tree ──Layouter──▶ boxes ──flatten──▶ DisplayList
                                                        ▲
                                              MathFont (OpenType MATH)
```

### Lexer (`lexer.rs`)

Pull-based. Whitespace and `%` comments are skipped in math mode. Control
sequences are letter runs or one non-letter character. The parser can ask for a
raw balanced group when it needs verbatim text (`\text{}`, environment names).

### Parser (`parser.rs`)

Recursive descent producing `ast::Node`. Nodes mirror TeX noads: every leaf
carries its atom class (Ord, Op, Bin, Rel, Open, Close, Punct, Inner) so layout
never inspects command names. Unknown commands are errors. The supported set is
tracked against KaTeX's function list.

### Font (`font.rs`)

Wraps `ttf-parser`. Exposes glyph metrics, the MATH constants as plain floats,
italic corrections, top accent attachment points, size variants and extensible
assemblies. All values are in font units; the layouter scales them.

### Layout (`layout.rs`)

Implements TeXbook Appendix G with OpenType MATH constants:

| Construct | Rule | Constants used |
| --- | --- | --- |
| Atom spacing | 20, table p.170 | mu = em/18 at the current style |
| Scripts | 18 | superscriptShiftUp(Cramped), subscriptShiftDown, superscriptBottomMin, subscriptTopMax, subSuperscriptGapMin, superscriptBottomMaxWithSubscript, baseline drops, spaceAfterScript |
| Limits | 13a | upper/lowerLimitGapMin, upper/lowerLimitBaselineRise/DropMin |
| Large operators | 13 | displayOperatorMinHeight, axisHeight |
| Fractions | 15 | fractionNumerator/Denominator(DisplayStyle)ShiftUp/Down, gap minima, fractionRuleThickness |
| Stacks (`\binom`) | 15 | stackTop/Bottom(DisplayStyle)Shift, stack(DisplayStyle)GapMin |
| Radicals | 11 | radical(DisplayStyle)VerticalGap, radicalRuleThickness, radicalExtraAscender, degree kerns and raise percent |
| Delimiters | 19 | axisHeight, TeX delimiterfactor 901 and shortfall 0.5 em |
| Accents | 12 | accentBaseHeight, topAccentAttachment |
| Over/underline | 9 | overbar/underbar gap, thickness, extra ascender/descender |

Boxes are TeX-style (width, height, depth, baseline at y = 0, y up). `flatten`
converts to y-down pixel coordinates once at the end.

**Italic correction convention.** Letters follow TeX: the superscript is
pushed right by the italic correction. Large operators follow the OpenType
convention (LuaTeX `\mathnolimitsmode=1`): their advance already covers the top
hook, so the subscript is pulled left instead. Latin Modern Math and every
other OpenType math font we have checked are designed for this.

**Extensible glyphs.** Pick the smallest pre-drawn size variant that reaches
the target, else build an assembly: repeat extender parts until the maximum
length with minimum connector overlap reaches the target, then solve for the
overlap that hits the target exactly, clamped to the connector lengths.

### Display list (`display.rs`)

```
DisplayList { width, ascent, descent, items: [Glyph { id, x, y, size, color } | Rule { x, y, width, height, color }] }
```

`id` is a glyph index in the engine's font, `size` is the em size in pixels.
This is intentionally the lowest common denominator every canvas API supports
directly: Android `Canvas.drawGlyphs`, Core Text `CTFontDrawGlyphs`, Flutter
`Canvas.drawRawAtlas`/paths, Skia `drawGlyphs`, HTML canvas paths.

## Backend contract

A backend needs three things:

1. The same font bytes the core was given.
2. A way to draw glyph `id` from that font at em size `size` with its origin at
   `(x, y)` on the baseline.
3. A filled rectangle.

`mathraster` is the reference backend and is also how CI verifies the core
without any device: it rasterizes with tiny-skia and diffs against golden PNGs.

## Testing strategy

- Unit tests on lexer, parser, symbol mapping and font access.
- Structural layout tests (baseline alignment, relative sizes, spacing
  inequalities) that do not depend on exact pixels.
- Golden-image tests over a corpus of real formulas. Any change in output must
  be reviewed as an image diff before the golden is updated.

## Decisions

- **Rust core, C ABI out.** Broadest reach: UniFFI for Kotlin/Swift, `dart:ffi`
  for Flutter, JSI/Nitro for React Native, wasm for the web.
- **OpenType MATH, not hard-coded TeX fontdimens.** Any compliant math font
  works: Latin Modern, STIX Two, XITS, Libertinus, Cambria, Fira Math.
- **Display list, not pixels.** Platform text stacks give GPU-accelerated,
  subpixel-correct glyphs; we only decide where they go.
- **Fail loudly on unknown commands.** A silently dropped `\foo` is a bug
  report that arrives months later from a user.
