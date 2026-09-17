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

### Macros (`macros.rs`)

Textual expansion before lexing: `\newcommand`, `\renewcommand`,
`\providecommand` and `\def` found in the source are removed and applied,
together with host-supplied definitions. Expansion is bounded so a recursive
macro is an error, not a hang.

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

**Math kerning.** When both the base and the script are single glyphs, the
MathKernInfo corners are sampled at the correction heights the OpenType spec
describes (base top-right at the script's bottom edge plus script bottom-left
at the base's top edge, and the mirror for subscripts) and the sum offsets the
script horizontally.

**Text shaping.** `\text{}` runs go through a small built-in shaper:
GSUB `liga` ligatures and GPOS `kern` pair adjustments read straight from
ttf-parser. Script styles substitute the font's `ssty` alternates, which is
what makes nested subscripts match LuaLaTeX.

**Extensible glyphs.** Pick the smallest pre-drawn size variant that reaches
the target, else build an assembly: repeat extender parts until the maximum
length with minimum connector overlap reaches the target, then solve for the
overlap that hits the target exactly, clamped to the connector lengths.

### Line breaking

`RenderOptions::line_break` gives the engine an available width. Candidates are
the positions before a Rel or Bin atom at the top level of the formula, carrying
TeX's own penalties (`\relpenalty` 500, `\binoppenalty` 700). A Knuth-Plass
style dynamic program over total demerits picks the split, so a formula breaks
into lines of similar length rather than one full line and a stub; a greedy
first fit would do the latter. The space at a break is discarded exactly as TeX
discards glue at a line break, the operator starts the continuation line as in
`multline` and `split`, and continuation lines are indented. Lines are stacked
with TeX's interline glue rule plus amsmath's `\jot`.

Nothing inside a `\left ... \right` group, a fraction or a radical is ever
broken, so a single wide fraction still overflows. That matches what an author
would do by hand.

### Display list (`display.rs`)

```
DisplayList { width, ascent, descent,
              items: [Glyph { id, x, y, size, color } | Rule { x, y, width, height, color } | Line { x1, y1, x2, y2, thickness, color }] }
```

`id` is a glyph index in the engine's font, `size` is the em size in pixels.
This is intentionally the lowest common denominator every canvas API supports
directly: Android `Canvas.drawGlyphs`, Core Text `CTFontDrawGlyphs`, Flutter
`Canvas.drawRawAtlas`/paths, Skia `drawGlyphs`, HTML canvas paths.

### C ABI (`crates/mathffi`)

`mathcore_ffi` exposes the same display list as a flat `MathItem` array plus
glyph outlines as a float command stream, so a binding never needs to parse
the font itself. Errors are returned as null plus a thread-local message.
Kotlin (JNA/UniFFI), Swift, Dart (`dart:ffi`) and JS (wasm) bindings all wrap
this one surface.

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
