/// One drawable item of a laid-out formula. Coordinates are pixels, y down,
/// origin at the top-left of the formula's bounding box.
sealed class MathItem {
  const MathItem(this.argb);

  /// 0xAARRGGBB.
  final int argb;
}

/// A glyph of the engine's font, drawn with its baseline origin at ([x], [y]) at [emSize] px.
class MathGlyph extends MathItem {
  const MathGlyph(this.id, this.x, this.y, this.emSize, super.argb);
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

/// A laid-out formula. The baseline sits at [ascent] from the top.
class MathLayout {
  const MathLayout({required this.width, required this.ascent, required this.descent, required this.items});
  final double width;
  final double ascent;
  final double descent;
  final List<MathItem> items;
  double get height => ascent + descent;
}

/// A glyph outline in font units, y up. Commands: 0 move (x y), 1 line (x y),
/// 2 quad (x1 y1 x y), 3 cubic (x1 y1 x2 y2 x y), 4 close.
class GlyphOutline {
  const GlyphOutline(this.commands);
  final List<double> commands;
}
