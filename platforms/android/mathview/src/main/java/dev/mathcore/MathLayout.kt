package dev.mathcore

/**
 * A laid-out formula: a flat list of glyphs, rules and lines in pixels.
 * Coordinates are y-down with the origin at the top-left of the bounding box;
 * the baseline sits at [ascent].
 */
class MathLayout internal constructor(private val data: FloatArray) {
    val width: Float get() = data[0]
    val ascent: Float get() = data[1]
    val descent: Float get() = data[2]
    val height: Float get() = ascent + descent
    val itemCount: Int get() = data[3].toInt()

    /** Visits every item. Glyph: (x, y, size). Rule: (x, y, w, h). Line: (x1, y1, x2, y2, thickness). */
    inline fun forEach(
        glyph: (id: Int, x: Float, y: Float, emSize: Float, argb: Int) -> Unit,
        rule: (x: Float, y: Float, w: Float, h: Float, argb: Int) -> Unit,
        line: (x1: Float, y1: Float, x2: Float, y2: Float, thickness: Float, argb: Int) -> Unit,
    ) {
        var i = 4
        repeat(itemCount) {
            val color = raw(i + 7).toRawBits()
            when (raw(i).toInt()) {
                0 -> glyph(raw(i + 1).toInt(), raw(i + 2), raw(i + 3), raw(i + 4), color)
                1 -> rule(raw(i + 2), raw(i + 3), raw(i + 4), raw(i + 5), color)
                else -> line(raw(i + 2), raw(i + 3), raw(i + 4), raw(i + 5), raw(i + 6), color)
            }
            i += 8
        }
    }

    @PublishedApi internal fun raw(index: Int): Float = data[index]
}

class MathParseException(message: String) : IllegalArgumentException(message)
