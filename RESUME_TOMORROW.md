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

On the Mac: Rust comes from Homebrew's rustup, so put
`/opt/homebrew/opt/rustup/bin` on `PATH`. Node is under nvm.

On Linux, installed and still there: Rust with the Android and wasm targets, `cargo-ndk`,
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
tools/corpus/run.sh                                      # 100k arXiv formulas, KaTeX diff, fuzzing (~2 min)
scripts/bench.sh                                         # side-by-side bench page with KaTeX and MathJax
UPDATE_GOLDEN=1 cargo test -p mathraster --test golden   # after an intended layout change
PATH=$HOME/.TinyTeX/bin/x86_64-linux:$PATH python3 tools/texcompare/compare.py   # against real TeX
scripts/build-android.sh && (cd platforms/android && ./gradlew :app:assembleDebug -PskipCargo)
```

Emulator: `setsid nohup android emulator start Medium_Phone_API_36.1 &`. It dies
if not fully detached. `android layout --device emulator-5554` dumps the
accessibility tree, which is how the spoken output was checked.

## What to do next

Done since the last plan: real-world corpus gate (98.2% of arXiv, KaTeX
94.1%), text mode, sizes, `\tag`, chemistry, AsciiMath, layout cache,
budgets on every ABI, speech verbosity and navigable speech trees, and the SDK
pipeline for C, Apple, Android, Flutter, web and React Native (docs/SDK.md),
each built and verified on this Mac, the mobile ones run on a simulator and
an emulator.

Next, biggest first:

1. **First release.** Run the Release workflow (needs `NPM_TOKEN`, and
   `RELEASE_TOKEN` plus pub.dev automated publishing for the pub packages).
   Maven Central needs signing keys; GitHub Packages works today.
2. **Speech in more languages, Nemeth braille.** The tree has the structure;
   the words are English only.
3. **Editing model** (cursor, selection, incremental relayout) for a math
   input control. Roughly triples scope; decide before more layout work.
4. **Remaining TeX:** `CD` diagrams, `\\` inside a brace group, `siunitx`.
5. **Throughput on pages with hundreds of formulas:** a glyph atlas in the
   platform renderers.

Machine notes (this Mac): Rust via Homebrew rustup, put
`/opt/homebrew/opt/rustup/bin` and `~/.cargo/bin` on PATH. Gradle's user
properties turn on the configuration cache, which Flutter's and React
Native's Gradle plugins reject; the pipeline turns it off for those builds.
CocoaPods 1.12 here cannot read Flutter's SwiftPM projects; the plugin
supports SwiftPM directly. React Native 0.87 wants Node 22 (here: 20; builds
still work).

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
- `cargo fmt` reflows long lines, so a scripted edit that matches a line
  written before formatting silently misses it. Check that every edit applied.
- Fabric attaches a component's event emitter after its first props: an
  event sent from the first `updateProps` is lost.
- CocoaPods rejects an xcframework mixing iOS and versioned macOS framework
  slices; ship one per platform.
- Byte offsets from the engine are UTF-8: every binding slices with them as
  bytes, never as UTF-16 indices.
