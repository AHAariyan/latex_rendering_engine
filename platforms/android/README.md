# mathcore for Android

Two modules:

- `mathview`: the library. Ships `libmathcore_android.so` (Rust engine + JNI
  bridge + bundled Latin Modern Math) and a small Kotlin API.
- `app`: a demo with a live TeX editor and a gallery of formulas.

## API

```kotlin
// Compose
MathText(latex = "\\frac{a}{b}", fontSize = 20.sp, color = Color.Black)

// Views
val view = MathView(context).apply { latex = "x^2"; textSizePx = 48f }

// Engine directly (one per font; MathEngine.shared uses the bundled font)
val layout = MathEngine.shared.render("\\sum_{i=1}^n i", fontSizePx = 40f, displayMode = true)
MathEngine.shared.draw(layout, canvas, left = 0f, top = 0f)
```

`render` throws `MathParseException` with the byte offset and reason for bad
input. Custom fonts: `MathEngine.fromFont(bytes)` accepts any OpenType font
with a MATH table.

## Build

```sh
# Rust side (needs cargo-ndk and the Android targets; see scripts/build-android.sh)
../../scripts/build-android.sh
# Gradle side
./gradlew :app:assembleDebug -PskipCargo
```

Without `-PskipCargo` the `mathview` module runs the cross-compile script
before every build.

## How drawing works

The native side returns a flat `FloatArray` per formula: glyph ids with
positions and em sizes, plus rules and lines. Glyph outlines are fetched once
per glyph id, cached as `android.graphics.Path` in font units, and drawn with
`translate + scale + drawPath` on the hardware-accelerated canvas. No
`Typeface` registration, no text APIs, no WebView.
