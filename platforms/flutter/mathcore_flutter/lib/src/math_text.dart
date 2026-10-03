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
/// The formula takes its colour and size from the surrounding
/// [DefaultTextStyle] unless [color] or [fontSize] are given, and follows the
/// user's text scaling, as text does.
///
/// TalkBack and VoiceOver read the whole formula, then let the user step
/// through its parts (terms, fractions, scripts), each outlined where drawn.
///
/// With [wrap] on, a formula too wide for the space the parent offers is broken
/// into lines before relations and binary operators, the way an author breaks a
/// long equation by hand. With it off the formula keeps its natural width,
/// which suits a horizontally scrollable row.
class MathText extends StatelessWidget {
  const MathText(
    this.latex, {
    super.key,
    this.fontSize,
    this.color,
    this.displayMode = true,
    this.wrap = true,
    this.macros = const {},
    this.speechVerbosity = SpeechVerbosity.brief,
    this.speechLanguage,
    this.errorBuilder,
    this.onTap,
  });

  final String latex;

  /// Em size in logical pixels, before text scaling. Defaults to the
  /// surrounding text style's size, or 18.
  final double? fontSize;

  /// Defaults to the surrounding text style's colour.
  final Color? color;
  final bool displayMode;
  final bool wrap;
  final Map<String, String> macros;

  /// How much scaffolding a screen reader hears.
  final SpeechVerbosity speechVerbosity;

  /// The language a screen reader hears the formula in (a BCP 47 tag);
  /// null follows the app's locale.
  final String? speechLanguage;

  /// Shown instead of the formula when parsing fails. Defaults to the message in red.
  final Widget Function(BuildContext, String message)? errorBuilder;

  /// Called with the smallest sub-expression under the finger. Its `start` and
  /// `end` are UTF-8 byte offsets into [latex]; see [MathRegion.textIn].
  final void Function(MathRegion region)? onTap;

  @override
  Widget build(BuildContext context) {
    if (!wrap) return _paint(context, null);
    return LayoutBuilder(
      builder: (context, constraints) => _paint(context, constraints.hasBoundedWidth ? constraints.maxWidth : null),
    );
  }

  Widget _paint(BuildContext context, double? maxWidth) {
    final style = DefaultTextStyle.of(context).style;
    final size = MediaQuery.textScalerOf(context).scale(fontSize ?? style.fontSize ?? 18);
    final ink = color ?? style.color ?? const Color(0xFF000000);
    final MathLayout layout;
    try {
      layout = MathCore.engine.render(
        latex,
        size,
        displayMode: displayMode,
        argb: ink.toARGB32(),
        macros: macros,
        maxWidth: maxWidth,
        hitTesting: true,
      );
    } on MathParseException catch (e) {
      final b = errorBuilder;
      return b != null
          ? b(context, e.message)
          : Text(e.message, style: const TextStyle(color: Color(0xFFB00020), fontSize: 12));
    }
    String? spoken;
    List<SpeechNode> parts = const [];
    final language = speechLanguage ?? Localizations.maybeLocaleOf(context)?.toLanguageTag();
    try {
      spoken = MathEngine.speechWith(latex, speechVerbosity, language: language);
      final tree = MathEngine.speechTree(latex, verbosity: speechVerbosity, language: language);
      if (tree.children.length > 1) parts = tree.children;
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
    final whole = Semantics(label: spoken, excludeSemantics: true, child: child);
    if (parts.isEmpty) return whole;
    // One semantics node per part, placed over it, for the screen reader to step through.
    return SizedBox(
      width: layout.width,
      height: layout.height,
      child: Stack(children: [
        whole,
        for (final part in parts)
          if (_bounds(layout, part) case final r?)
            Positioned.fromRect(rect: r, child: Semantics(label: part.announcement, container: true, child: const SizedBox.expand())),
      ]),
    );
  }

  static Rect? _bounds(MathLayout layout, SpeechNode part) {
    final regions = layout.highlight(part.start, part.end);
    if (regions.isEmpty) return null;
    var r = Rect.fromLTWH(regions.first.x, regions.first.y, regions.first.width, regions.first.height);
    for (final g in regions.skip(1)) {
      r = r.expandToInclude(Rect.fromLTWH(g.x, g.y, g.width, g.height));
    }
    return r;
  }
}
