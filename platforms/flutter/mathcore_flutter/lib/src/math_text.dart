import 'package:flutter/widgets.dart';
import 'package:mathcore_dart/mathcore_dart.dart';

import 'math_painter.dart';

/// Process-wide engine and path cache for the bundled font.
class MathCore {
  MathCore._();
  static MathEngine? _engine;
  static GlyphPathCache? _cache;
  static MathEngine get engine => _engine ??= MathEngine.bundled();
  static GlyphPathCache get cache => _cache ??= GlyphPathCache(engine);
}

/// Typesets [latex] natively. The widget sizes itself to the formula in
/// logical pixels; the layout is computed at device pixel ratio for crisp glyphs.
class MathText extends StatelessWidget {
  const MathText(
    this.latex, {
    super.key,
    this.fontSize = 18,
    this.color = const Color(0xFF000000),
    this.displayMode = true,
    this.macros = const {},
    this.errorBuilder,
  });

  final String latex;
  /// Em size in logical pixels.
  final double fontSize;
  final Color color;
  final bool displayMode;
  final Map<String, String> macros;
  /// Shown instead of the formula when parsing fails. Defaults to the message in red.
  final Widget Function(BuildContext, String message)? errorBuilder;

  @override
  Widget build(BuildContext context) {
    final MathLayout layout;
    try {
      layout = MathCore.engine.render(latex, fontSize, displayMode: displayMode, argb: color.toARGB32(), macros: macros);
    } on MathParseException catch (e) {
      final b = errorBuilder;
      return b != null
          ? b(context, e.message)
          : Text(e.message, style: const TextStyle(color: Color(0xFFB00020), fontSize: 12));
    }
    return CustomPaint(
      size: Size(layout.width, layout.height),
      painter: MathPainter(layout, MathCore.cache),
    );
  }
}
