# mathcore_flutter

Native TeX math for Flutter on Android, iOS and macOS: the mathcore engine
over `dart:ffi`. No WebView, no fonts to bundle, no platform channels.

```dart
MathText(r'x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}')
MathText(r'\ce{2H2 + O2 -> 2H2O}', fontSize: 22)
MathText(MathEngine.asciimathToTex('sum_(i=1)^n i^2'))
```

- Takes its colour and size from the surrounding `DefaultTextStyle`, and
  follows the user's text scaling.
- Breaks a formula too wide for its parent into lines; `wrap: false` keeps
  it on one line for a horizontally scrolling row.
- TalkBack and VoiceOver read the formula as a sentence, then let the user
  step through its parts. `speechVerbosity` sets how much they hear.
- `onTap` reports the sub-expression under the finger.
- Parse errors show as a red message, or through `errorBuilder`.

`MathCore.engine` is the shared engine for custom painting with
`MathPainter`; `mathcore_dart` (re-exported) has the full API: speech trees,
MathML, AsciiMath, budgets for untrusted input.

## Native library

Shipped prebuilt: `libmathcore_ffi.so` for arm64-v8a, armeabi-v7a and x86_64,
and `MathCoreFFI.xcframework`, a dynamic framework for iOS and macOS that
works with both Swift Package Manager and CocoaPods. Being dynamic, it keeps
working in release builds whatever the app's symbol stripping settings.

Built from source by `cargo xtask sdk flutter` in the mathcore repository.
