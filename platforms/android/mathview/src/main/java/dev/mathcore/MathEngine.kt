package dev.mathcore

import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Matrix
import android.graphics.Paint
import android.graphics.Path
import java.io.Closeable

/**
 * A TeX math typesetting engine backed by the native `mathcore` library.
 *
 * One engine per font. [shared] is the process-wide engine using the bundled
 * Latin Modern Math font; it is what [MathView] and [MathText] use. Glyph
 * outlines are cached as [Path]s in font units so drawing is one
 * translate+scale+drawPath per glyph on the hardware-accelerated canvas.
 */
class MathEngine private constructor(private var handle: Long) : Closeable {
    companion object {
        /** The primary plus a small number of fallbacks; more than this is not a font stack. */
        private const val MAX_FONTS = 8

        /** Engine with the bundled font, created on first use. */
        val shared: MathEngine by lazy { bundled() }

        fun bundled(): MathEngine = MathEngine(check(NativeBridge.createBundled()))

        /** Engine for any OpenType font with a MATH table. */
        fun fromFont(fontBytes: ByteArray): MathEngine = MathEngine(check(NativeBridge.create(fontBytes, null)))

        /**
         * Engine with a math font and a font for `\text{}`. Pass null for the
         * math font to keep the bundled one. Use this to set prose in the
         * application's own face while the maths stays in the math font.
         */
        fun withFonts(mathFont: ByteArray? = null, textFont: ByteArray? = null): MathEngine =
            MathEngine(check(NativeBridge.create(mathFont, textFont)))

        private fun check(handle: Long): Long {
            if (handle == 0L) throw IllegalStateException(NativeBridge.lastError() ?: "cannot create math engine")
            return handle
        }
    }

    private val unitsPerEm = FloatArray(MAX_FONTS) { NativeBridge.unitsPerEm(handle, it) }

    /** Font units per em of one font of the chain; a fallback may differ. */
    fun unitsPerEm(font: Int = 0): Float = unitsPerEm.getOrElse(font) { 0f }

    /** Keyed by font and glyph, since a formula may draw from more than one font. */
    private val paths = HashMap<Int, Path?>()
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.FILL }
    private val matrix = Matrix()

    /**
     * Lays out [tex]. [fontSizePx] is the em size in pixels. When [maxWidthPx]
     * is positive, a formula wider than that is broken into lines before
     * relations and binary operators. With [hitTesting] on, the layout also
     * records which part of the source each piece came from, so a tap can be
     * mapped back to a sub-expression.
     * @throws MathParseException when the source cannot be parsed.
     */
    @Synchronized
    fun render(
        tex: String,
        fontSizePx: Float,
        displayMode: Boolean = true,
        color: Int = Color.BLACK,
        macros: Map<String, String> = emptyMap(),
        maxWidthPx: Float = 0f,
        hitTesting: Boolean = false,
    ): MathLayout {
        val macroText = if (macros.isEmpty()) null else macros.entries.joinToString("\n") { "${it.key}=${it.value}" }
        val data = NativeBridge.render(handle, tex, fontSizePx, displayMode, color, macroText, maxWidthPx, hitTesting)
            ?: throw MathParseException(NativeBridge.lastError() ?: "render failed")
        return MathLayout(data)
    }

    /** Outline of a glyph in font units, y down. Null when the glyph has no outline. */
    @Synchronized
    fun glyphPath(font: Int, glyph: Int): Path? = paths.getOrPut((font shl 16) or glyph) {
        val cmds = NativeBridge.glyphOutline(handle, font, glyph) ?: return@getOrPut null
        val p = Path()
        var i = 0
        while (i < cmds.size) {
            when (cmds[i].toInt()) {
                0 -> { p.moveTo(cmds[i + 1], -cmds[i + 2]); i += 3 }
                1 -> { p.lineTo(cmds[i + 1], -cmds[i + 2]); i += 3 }
                2 -> { p.quadTo(cmds[i + 1], -cmds[i + 2], cmds[i + 3], -cmds[i + 4]); i += 5 }
                3 -> { p.cubicTo(cmds[i + 1], -cmds[i + 2], cmds[i + 3], -cmds[i + 4], cmds[i + 5], -cmds[i + 6]); i += 7 }
                else -> { p.close(); i += 1 }
            }
        }
        p
    }

    /** Draws [layout] with its top-left corner at ([left], [top]). */
    @Synchronized
    fun draw(layout: MathLayout, canvas: Canvas, left: Float = 0f, top: Float = 0f) {
        layout.forEach(
            glyph = { font, id, x, y, em, argb ->
                val path = glyphPath(font, id)
                if (path != null) {
                    val k = em / unitsPerEm(font)
                    matrix.reset()
                    matrix.setScale(k, k)
                    matrix.postTranslate(left + x, top + y)
                    paint.color = argb
                    canvas.save()
                    canvas.concat(matrix)
                    canvas.drawPath(path, paint)
                    canvas.restore()
                }
            },
            rule = { x, y, w, h, argb ->
                paint.color = argb
                canvas.drawRect(left + x, top + y, left + x + w, top + y + h, paint)
            },
            line = { x1, y1, x2, y2, thickness, argb ->
                paint.color = argb
                paint.style = Paint.Style.STROKE
                paint.strokeWidth = thickness
                canvas.drawLine(left + x1, top + y1, left + x2, top + y2, paint)
                paint.style = Paint.Style.FILL
            },
        )
    }

    @Synchronized
    override fun close() {
        if (handle != 0L) {
            NativeBridge.destroy(handle)
            handle = 0L
        }
    }
}
