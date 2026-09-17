import 'package:flutter/semantics.dart';
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

/// Typesets [latex] natively and sizes itself to the formula.
///
/// TalkBack and VoiceOver read the formula aloud: the widget carries a spoken
/// rendering as its semantics label, so `x^2` is announced as "x squared".
///
/// With [wrap] on, a formula too wide for the space the parent offers is broken
/// into lines before relations and binary operators, the way an author breaks a
/// long equation by hand. With it off the formula keeps its natural width,
/// which suits a horizontally scrollable row.
class MathText extends StatelessWidget {
  const MathText(
    this.latex, {
    super.key,
    this.fontSize = 18,
    this.color = const Color(0xFF000000),
    this.displayMode = true,
    this.wrap = true,
    this.macros = const {},
    this.errorBuilder,
    this.onTap,
  });

  final String latex;
  /// Em size in logical pixels.
  final double fontSize;
  final Color color;
  final bool displayMode;
  final bool wrap;
  final Map<String, String> macros;
  /// Shown instead of the formula when parsing fails. Defaults to the message in red.
  final Widget Function(BuildContext, String message)? errorBuilder;

  /// Called with the smallest sub-expression under the finger. Its `start` and
  /// `end` index into [latex]. Providing it turns hit testing on.
  final void Function(MathRegion region)? onTap;

  @override
  Widget build(BuildContext context) {
    if (!wrap) return _paint(context, null);
    return LayoutBuilder(
      builder: (context, constraints) => _paint(context, constraints.hasBoundedWidth ? constraints.maxWidth : null),
    );
  }

  Widget _paint(BuildContext context, double? maxWidth) {
    final MathLayout layout;
    try {
      layout = MathCore.engine.render(
        latex,
        fontSize,
        displayMode: displayMode,
        argb: color.toARGB32(),
        macros: macros,
        maxWidth: maxWidth,
        hitTesting: onTap != null,
      );
    } on MathParseException catch (e) {
      final b = errorBuilder;
      return b != null
          ? b(context, e.message)
          : Text(e.message, style: const TextStyle(color: Color(0xFFB00020), fontSize: 12));
    }
    String? spoken;
    try {
      spoken = MathEngine.speech(latex);
    } on MathParseException {
      spoken = null;
    }
    Widget child = CustomPaint(
      size: Size(layout.width, layout.height),
      painter: MathPainter(layout, MathCore.cache),
    );
    final tap = onTap;
    if (tap != null) {
      child = GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapDown: (d) {
          final r = layout.hitNearest(d.localPosition.dx, d.localPosition.dy);
          if (r != null) tap(r);
        },
        child: child,
      );
    }
    return Semantics(label: spoken, excludeSemantics: true, child: child);
  }
}
