# Resume here

State of the engine, how to get the machine ready again, and what to do next.
Last session ended at commit `bc0c939`, everything pushed to `main`.

## What exists

A native TeX math engine: one Rust core that turns TeX into a display list
(glyph ids with positions, rules, lines), and thin per-platform backends that
draw it. No WebView anywhere.

| Crate | What |
| --- | --- |
| `crates/mathcore` | Lexer, parser, macros, OpenType MATH font layer, layout, line breaking, accessibility, hit testing, budgets |
| `crates/mathraster` | tiny-skia PNG rasterizer and SVG writer, used by the golden tests |
| `crates/mathcli` | Command line renderer |
| `crates/mathffi` | C ABI, `include/mathcore.h` |
| `crates/mathjni` | JNI bridge for Android |
| `crates/mathwasm` | wasm-bindgen binding |

| Platform | State |
| --- | --- |
| Android (`platforms/android`) | Built and verified on an emulator |
| Web (`platforms/web`) | Built, tested in Node |
| Dart (`platforms/dart`) | Built, tested |
| Flutter (`platforms/flutter`) | Written, never compiled: needs the Flutter SDK |
| iOS (`platforms/ios`) | Written, never compiled: needs a Mac with Xcode |

Quality gates, all green at the last commit:

- 170 golden images, 58 formulas across three fonts.
- Comparison against real LuaLaTeX (`tools/texcompare/compare.py`): every corpus
  formula within 5% in both dimensions, except display `\binom` where LuaLaTeX
  itself departs from TeX's rule.
- Cross-binding parity: 60 formulas, 1060 items identical between the core,
  WebAssembly and Dart (`scripts/parity.sh`).
- Fuzzing: 20,000 random inputs plus pathological cases, no panics, small-stack safe.
- clippy at deny-warnings, rustfmt clean.

## Getting the machine ready

Installed and still there: Rust with the Android and wasm targets, `cargo-ndk`,
`wasm-pack`, Android SDK and NDK 28 at `~/Android/Sdk`, JDK 21, TinyTeX at
`~/.TinyTeX`, Node.

Two things live in the session scratchpad and are gone; recreate them if needed:

```sh
# Dart SDK, for the Dart package tests
curl -sL -o dart.zip https://storage.googleapis.com/dart-archive/channels/stable/release/latest/sdk/dartsdk-linux-x64-release.zip
unzip -q dart.zip            # then use ./dart-sdk/bin/dart

# fontTools, for the font subsetting scripts
uv venv ft && uv pip install --python ./ft/bin/python fonttools brotli
```

LaTeX comparison needs `export PATH=$HOME/.TinyTeX/bin/x86_64-linux:$PATH`.

## Running everything

```sh
cargo test --workspace                                   # unit, golden, subset, parity reference
cargo test --release -p mathcore --test robustness       # fuzz and budgets, release stack budget
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p mathcore --features complex-text           # the optional shaper
scripts/parity.sh                                        # wasm and Dart against the core
UPDATE_GOLDEN=1 cargo test -p mathraster --test golden   # after an intended layout change
PATH=$HOME/.TinyTeX/bin/x86_64-linux:$PATH python3 tools/texcompare/compare.py   # against real TeX
scripts/build-android.sh && (cd platforms/android && ./gradlew :app:assembleDebug -PskipCargo)
```

Emulator: `setsid nohup android emulator start Medium_Phone_API_36.1 &`. It dies
if not fully detached. `android layout --device emulator-5554` dumps the
accessibility tree, which is how the spoken output was checked.

## What to do next

The gap list against a world-class tool, biggest first. The first three are
done; work down from there.

1. ~~Hit testing and selection~~ done.
2. ~~Command coverage~~ done: 585 symbols, KaTeX parity where it matters.
3. ~~Text handling~~ done: text font role, fallback, right-to-left, optional shaper.
4. **Accessibility depth.** Ours emits MathML and one spoken sentence. The
   benchmark is MathJax's Speech Rule Engine: verbosity levels, more languages,
   sub-expression navigation, synchronized highlighting, Nemeth braille.
5. **Test corpus size.** We validate 58 formulas. Real engines validate against
   hundreds of thousands from arXiv and Wikipedia. Pulling a few thousand and
   running them through the parser would find the real gaps fast.
6. **Input formats.** TeX only. No MathML in, no AsciiMath, no Word OMML.
7. **Throughput at scale.** Every glyph is a path fill. A glyph atlas with
   batched draws would matter on a page with hundreds of formulas. No layout
   cache, no incremental relayout.
8. **Ecosystem.** Nothing is published: Maven, npm, pub.dev, SwiftPM. No docs
   site, no API stability promise, no continuous fuzzing, no security review.
9. **Platform reach.** Compile the Flutter and iOS packages. React Native.
   Desktop toolkits.

Smaller open items: `mhchem` for chemistry, `\tag` display, `\hdashline`,
`\let`/`\expandafter`/`\csname`, exposing `Budget` through the C ABI, and a
full bidirectional algorithm for mixed-direction prose.

Needs your decision: whether to build an editing model (cursor, selection,
incremental relayout) for a math input control. It roughly triples the scope
and shapes the tree design, so it should be decided before more layout work.

## Things that bit us, so they do not bite again

- Patches against Rust sources must tolerate rustfmt's line wrapping. A silently
  skipped patch to the flat serializer cost a round of device debugging. Verify
  every patch applied.
- Gradle 9's configuration cache rejects any task lambda that touches the
  `project` object. Read properties at configuration time.
- tiny-skia's debug build asserts on sub-pixel `fill_rect` and on hairline
  strokes. Rules and lines are filled as paths for that reason.
- fontTools drops MATH records for glyphs reachable only through GSUB, which is
  exactly the `ssty` script alternates. `tools/subset/repair_math.py` restores
  them; the subset test will catch it if that regresses.
- A reference file that is not regenerated after a font change fails parity
  loudly. That is the harness working, not a bug.
