import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:mathcore_dart/mathcore_dart.dart';

/// Caches [ui.Path]s per glyph id for one [MathEngine].
class GlyphPathCache {
  GlyphPathCache(this.engine);
  final MathEngine engine;
  final Map<int, ui.Path?> _paths = {};

  /// Glyph outline in font units, y down.
  ui.Path? path(int glyph) => _paths.putIfAbsent(glyph, () {
        final o = engine.glyphOutline(glyph);
        if (o == null) return null;
        final p = ui.Path();
        final c = o.commands;
        for (var i = 0; i < c.length;) {
          switch (c[i].toInt()) {
            case 0:
              p.moveTo(c[i + 1], -c[i + 2]);
              i += 3;
            case 1:
              p.lineTo(c[i + 1], -c[i + 2]);
              i += 3;
            case 2:
              p.quadraticBezierTo(c[i + 1], -c[i + 2], c[i + 3], -c[i + 4]);
              i += 5;
            case 3:
              p.cubicTo(c[i + 1], -c[i + 2], c[i + 3], -c[i + 4], c[i + 5], -c[i + 6]);
              i += 7;
            default:
              p.close();
              i += 1;
          }
        }
        return p;
      });
}

/// Paints a [MathLayout] with its top-left corner at [offset].
class MathPainter extends CustomPainter {
  MathPainter(this.layout, this.cache, {this.offset = Offset.zero});
  final MathLayout layout;
  final GlyphPathCache cache;
  final Offset offset;

  static void draw(Canvas canvas, MathLayout layout, GlyphPathCache cache, Offset offset) {
    final paint = Paint()..isAntiAlias = true;
    final upem = cache.engine.unitsPerEm;
    for (final item in layout.items) {
      paint.color = Color(item.argb);
      switch (item) {
        case MathGlyph g:
          final path = cache.path(g.id);
          if (path == null) break;
          final k = g.emSize / upem;
          canvas.save();
          canvas.translate(offset.dx + g.x, offset.dy + g.y);
          canvas.scale(k, k);
          paint.style = PaintingStyle.fill;
          canvas.drawPath(path, paint);
          canvas.restore();
        case MathRule r:
          paint.style = PaintingStyle.fill;
          canvas.drawRect(Rect.fromLTWH(offset.dx + r.x, offset.dy + r.y, r.width, r.height), paint);
        case MathLine l:
          paint
            ..style = PaintingStyle.stroke
            ..strokeWidth = l.thickness;
          canvas.drawLine(Offset(offset.dx + l.x1, offset.dy + l.y1), Offset(offset.dx + l.x2, offset.dy + l.y2), paint);
      }
    }
  }

  @override
  void paint(Canvas canvas, Size size) => draw(canvas, layout, cache, offset);

  @override
  bool shouldRepaint(MathPainter old) => old.layout != layout || old.offset != offset || old.cache != cache;
}
