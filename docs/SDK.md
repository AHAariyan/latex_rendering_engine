# SDKs

One Rust engine, packaged for every platform by one pipeline. Every SDK is
built from the same commit at the same version and verified before it is
packaged.

```sh
cargo xtask sdk <platform>... [--no-verify]   # build, verify, package into dist/
cargo xtask sdk all
cargo xtask version [<new>] [--check]         # the one version every SDK carries
cargo xtask ci                                # what CI runs on every push
```

| Platform | Package | Built on | Verified by |
| --- | --- | --- | --- |
| `c` | `mathcore-c-<v>-<target>.tar.gz`: header, static + shared libs, pkg-config, CMake | any | an example compiled with `-Werror` and run |
| `apple` | `MathCore` Swift package + `MathCoreFFI.xcframework` (iOS, simulator, macOS); CocoaPods spec | macOS | `swift test`; `xcodebuild` for iOS simulator and device |
| `android` | `dev.mathcore:mathcore-android` AAR (arm64-v8a, armeabi-v7a, x86_64) | macOS, Linux | JVM unit tests against the real engine; lint; demo app build |
| `flutter` | `mathcore_dart` + `mathcore_flutter` (Android, iOS, macOS) | macOS | dart/flutter analyze and tests; example app for Android, iOS, macOS; integration test inside the macOS app; pub validation |
| `web` | `mathcore` npm package | any | the tarball installed and used from Node; types compiled under `--strict` |
| `react-native` | `react-native-mathcore` | macOS | TypeScript and the native views compiled in an example app |

Requirements: Rust (rustup), plus per platform Xcode, the Android SDK and NDK
with `cargo install cargo-ndk`, Flutter, Node 18+ with `cargo install
wasm-pack`. The pipeline installs Rust targets itself and says what is missing.

## Using them

### Swift (iOS, macOS)

```swift
.package(url: "https://github.com/AHAariyan/latex_rendering_engine", from: "1.0.0")
```

```swift
import MathCore
MathText(#"x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}"#, fontSize: 22)   // SwiftUI
let view = MathView(); view.latex = #"\ce{2H2 + O2 -> 2H2O}"#      // UIKit
```

### Android

```kotlin
implementation("dev.mathcore:mathcore-android:1.0.0")
```

```kotlin
MathText(latex = "\\int_0^1 x^2\\,dx", fontSize = 20.sp)            // Compose
MathView(context).apply { latex = "x^2" }                           // Views
```

### Flutter

```yaml
dependencies:
  mathcore_flutter: ^1.0.0
```

```dart
MathText(r'\sum_{n=1}^\infty \frac{1}{n^2} = \frac{\pi^2}{6}')
```

### Web

```sh
npm install mathcore
```

```html
<script type="module">import "mathcore/element";</script>
<math-tex display>e^{i\pi} + 1 = 0</math-tex>
```

### React Native

```sh
npm install react-native-mathcore
```

```tsx
<MathText latex="\\frac{a}{b}" fontSize={20} />
```

### C and other languages

`include/mathcore.h` documents the whole API. Link the static library (or the
shared one) and draw the display list with any canvas; glyph outlines come
from the engine, so the host never parses a font. Anything with a C FFI
works: C++, Python (ctypes, cffi), Go (cgo), C# (P/Invoke), Java (FFM).

## What every SDK offers

| | C | Swift | Kotlin | Dart/Flutter | Web | RN |
| --- | --- | --- | --- | --- | --- | --- |
| TeX, chemistry (`\ce`), text mode | yes | yes | yes | yes | yes | yes |
| AsciiMath input | yes | yes | yes | yes | yes | yes |
| Line breaking to a width | yes | yes | yes | yes | yes | yes |
| Hit testing, highlight | yes | yes | yes | yes | yes | taps |
| Speech, verbosity, speech tree | yes | yes | yes | yes | yes | yes |
| Speech in 35 languages | yes | device | device | app locale | page `lang` | device |
| Text in any script (bidi, full shaping) | yes | yes | yes | yes | yes | yes |
| Fonts for other scripts found automatically | `math_engine_use_system_fonts` | CoreText | system fonts | font folders | Noto CDN | native |
| Nemeth braille | yes | yes | yes | yes | yes | yes |
| Screen-reader navigation of parts | (host) | VoiceOver | TalkBack | both | aria-label | both |
| Budgets for untrusted input | yes | yes | yes | yes | (defaults) | (defaults) |
| Layout cache | yes | yes | yes | yes | yes | yes |

Every SDK can also add a font of your own (`addFont` / `math_engine_add_font`)
and list what a formula still lacks (`missingCharacters` / `missingChars`).
Automatic fonts are on by default: `usesSystemFonts = false` on Swift,
Kotlin and Dart, `setFontSource(null)` on the web.

## Releases

Run the **Release** workflow with a version. It builds and verifies every
SDK at that version, commits the SwiftPM `Package.swift` with the
xcframework's checksum, tags `v<version>`, creates a GitHub release with every
archive and `SHA256SUMS`, publishes the npm package (with `NPM_TOKEN`) and
the Android AAR to GitHub Packages. The tag triggers **Publish to pub.dev**
(automated publishing; push the tag with `RELEASE_TOKEN` so it triggers).
Versions with a pre-release suffix (`1.1.0-beta.1`) publish as pre-releases.

Engine panics never reach the host: SDKs are built with the `sdk` profile
(unwinding), and every native entry point turns a panic into an error return.

## Compatibility

- iOS 13+ (SwiftUI `MathText` 15+), macOS 11+ (10.14 for Flutter).
- Android API 24+.
- Flutter 3.24+, Dart 3.5+.
- Node 18+, every browser with WebAssembly.
- The C ABI only grows within a major version.
