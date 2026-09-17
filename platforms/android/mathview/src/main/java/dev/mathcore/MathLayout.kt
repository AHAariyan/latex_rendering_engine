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

    private val regionBase: Int get() = 4 + itemCount * 8

    /** Empty unless the layout was rendered with hit testing on. */
    val regions: List<MathRegion> by lazy {
        if (regionBase >= data.size) return@lazy emptyList()
        val n = data[regionBase].toInt()
        (0 until n).map { i ->
            val b = regionBase + 1 + i * 7
            MathRegion(
                start = data[b].toInt(),
                end = data[b + 1].toInt(),
                x = data[b + 2],
                y = data[b + 3],
                width = data[b + 4],
                height = data[b + 5],
                depth = data[b + 6].toInt(),
            )
        }
    }

    /** Every region containing the point, outermost first. */
    fun hit(x: Float, y: Float): List<MathRegion> = regions.filter { it.contains(x, y) }.sortedByDescending { it.width * it.height }

    /** The smallest piece of source under the point. */
    fun hitTest(x: Float, y: Float): MathRegion? = hit(x, y).lastOrNull()

    /**
     * The piece of source under the point, or the closest one when the point
     * falls in the space between atoms. A tap is never pixel-exact, so this is
     * what a view should call.
     */
    fun hitNearest(x: Float, y: Float): MathRegion? =
        hitTest(x, y) ?: regions.minWithOrNull(
            compareBy({ it.distanceTo(x, y) }, { it.width * it.height }),
        )
}

/**
 * Where a piece of the source ended up on screen. Regions nest, so a tap
 * usually falls in several and the smallest is the innermost sub-expression.
 */
data class MathRegion(
    /** Byte range of the source that produced this piece. */
    val start: Int,
    val end: Int,
    val x: Float,
    val y: Float,
    val width: Float,
    val height: Float,
    /** Nesting level; 0 is a top-level atom. */
    val depth: Int,
) {
    fun contains(px: Float, py: Float): Boolean = px >= x && px <= x + width && py >= y && py <= y + height

    /** Distance from a point to this rectangle; zero when inside. */
    fun distanceTo(px: Float, py: Float): Float {
        val dx = maxOf(x - px, px - (x + width), 0f)
        val dy = maxOf(y - py, py - (y + height), 0f)
        return kotlin.math.sqrt(dx * dx + dy * dy)
    }

    /** Slices the source this region came from. */
    fun textIn(latex: String): String = latex.substring(start.coerceIn(0, latex.length), end.coerceIn(0, latex.length))
}

class MathParseException(message: String) : IllegalArgumentException(message)
