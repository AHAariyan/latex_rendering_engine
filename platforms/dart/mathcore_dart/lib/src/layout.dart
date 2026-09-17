import 'dart:math' as math;

/// One drawable item of a laid-out formula. Coordinates are pixels, y down,
/// origin at the top-left of the formula's bounding box.
sealed class MathItem {
  const MathItem(this.argb);

  /// 0xAARRGGBB.
  final int argb;
}

/// A glyph of the engine's font, drawn with its baseline origin at ([x], [y]) at [emSize] px.
class MathGlyph extends MathItem {
  const MathGlyph(this.font, this.id, this.x, this.y, this.emSize, super.argb);

  /// Which font of the engine's chain this glyph belongs to; 0 is the primary.
  final int font;
  final int id;
  final double x, y, emSize;
}

/// A filled rectangle.
class MathRule extends MathItem {
  const MathRule(this.x, this.y, this.width, this.height, super.argb);
  final double x, y, width, height;
}

/// A stroked straight line.
class MathLine extends MathItem {
  const MathLine(this.x1, this.y1, this.x2, this.y2, this.thickness, super.argb);
  final double x1, y1, x2, y2, thickness;
}

/// Where a piece of the source ended up on screen. Regions nest, so a point
/// usually falls in several and the smallest is the innermost sub-expression.
class MathRegion {
  const MathRegion(this.start, this.end, this.x, this.y, this.width, this.height, this.depth);

  /// Byte range of the source that produced this piece.
  final int start, end;
  final double x, y, width, height;

  /// Nesting level; 0 is a top-level atom.
  final int depth;

  bool contains(double px, double py) => px >= x && px <= x + width && py >= y && py <= y + height;

  /// Distance from a point to this rectangle; zero when inside.
  double distanceTo(double px, double py) {
    final dx = [x - px, px - (x + width), 0.0].reduce((a, b) => a > b ? a : b);
    final dy = [y - py, py - (y + height), 0.0].reduce((a, b) => a > b ? a : b);
    return math.sqrt(dx * dx + dy * dy);
  }

  double get area => width * height;

  /// Slices the source this region came from.
  String textIn(String latex) => latex.substring(start.clamp(0, latex.length), end.clamp(0, latex.length));
}

/// A laid-out formula. The baseline sits at [ascent] from the top.
class MathLayout {
  const MathLayout({
    required this.width,
    required this.ascent,
    required this.descent,
    required this.items,
    this.regions = const [],
  });
  final double width;
  final double ascent;
  final double descent;
  final List<MathItem> items;

  /// Empty unless the layout was rendered with hit testing on.
  final List<MathRegion> regions;
  double get height => ascent + descent;

  /// Every region containing the point, outermost first.
  List<MathRegion> hit(double x, double y) {
    final found = regions.where((r) => r.contains(x, y)).toList()..sort((a, b) => b.area.compareTo(a.area));
    return found;
  }

  /// The smallest piece of source under the point.
  MathRegion? hitTest(double x, double y) {
    final found = hit(x, y);
    return found.isEmpty ? null : found.last;
  }

  /// The piece of source under the point, or the closest one when the point
  /// falls in the space between atoms. A tap is never pixel-exact, so this is
  /// what a widget should call.
  MathRegion? hitNearest(double x, double y) {
    final exact = hitTest(x, y);
    if (exact != null || regions.isEmpty) return exact;
    final sorted = regions.toList()
      ..sort((a, b) {
        final d = a.distanceTo(x, y).compareTo(b.distanceTo(x, y));
        return d != 0 ? d : a.area.compareTo(b.area);
      });
    return sorted.first;
  }
}

/// A glyph outline in font units, y up. Commands: 0 move (x y), 1 line (x y),
/// 2 quad (x1 y1 x y), 3 cubic (x1 y1 x2 y2 x y), 4 close.
class GlyphOutline {
  const GlyphOutline(this.commands);
  final List<double> commands;
}
