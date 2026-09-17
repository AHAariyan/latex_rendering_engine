# mathcore_flutter

Native TeX math rendering for Flutter over `dart:ffi`. No WebView.

```dart
MathText(r'\frac{a}{b} + \sqrt{x^2 + y^2}', fontSize: 24, color: Colors.black)
```

The widget sizes itself to the formula. Parse errors render as a red message,
or use `errorBuilder`. `MathCore.engine` exposes the engine for custom
painting through `MathPainter`.

## Native library

This is an FFI plugin: it ships prebuilt `mathcore_ffi` binaries.

- Android: `scripts/build-android-ffi.sh` writes `libmathcore_ffi.so` per ABI
  into `android/src/main/jniLibs/`.
- iOS: `scripts/build-ios.sh` produces `MathCoreFFI.xcframework`; copy it into
  `ios/`.

The pure Dart layer (`mathcore_dart`) is tested on the host against the
desktop build of the same library and needs no Flutter.
