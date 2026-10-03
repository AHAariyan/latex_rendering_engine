# mathcore_dart

Pure Dart binding (`dart:ffi`) for mathcore, a native TeX math typesetting
engine. No Flutter dependency: use it on servers, in CLIs, or under your own
renderer. For the Flutter widget, use
[mathcore_flutter](https://pub.dev/packages/mathcore_flutter), which ships
the native library for Android, iOS and macOS.

```dart
import 'package:mathcore_dart/mathcore_dart.dart';

final engine = MathEngine.bundled();
final layout = engine.render(r'x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}', 32);
// layout.items: glyph ids with positions, rules and lines, in pixels.
// engine.glyphOutline(font, id): the glyph's outline, to draw it yourself.

MathEngine.speech(r'x^2 + y^2 = z^2');   // "x squared plus y squared equals z squared"
MathEngine.speechTree(r'\frac{a+b}{c}'); // parts with source ranges, for screen readers
MathEngine.asciimathToTex('sqrt(x)/2');  // AsciiMath input
```

Supports KaTeX's command set and more: text mode with `$...$`, sizes, `\tag`,
chemistry with `\ce` and `\pu`, line breaking to a width, hit testing, MathML.

Outside Flutter, point `MathEngine.libraryPath` at the native library for
your platform (the mathcore C SDK, or a build of `crates/mathffi`).
