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

        /** AsciiMath (`sum_(i=1)^n i^2`) translated to TeX for [render]. */
        fun asciimathToTex(source: String): String =
            NativeBridge.asciimathToTex(source) ?: throw MathParseException(NativeBridge.lastError() ?: "asciimath failed")

        private fun check(handle: Long): Long {
            if (handle == 0L) throw IllegalStateException(NativeBridge.lastError() ?: "cannot create math engine")
            return handle
        }
    }

    private val unitsPerEm = HashMap<Int, Float>()

    /** Font units per em of one font of the chain; a fallback may differ. */
    @Synchronized
    fun unitsPerEm(font: Int = 0): Float = unitsPerEm.getOrPut(font) { NativeBridge.unitsPerEm(handle, font) }

    /**
     * Before each render, find system fonts for characters the engine's own
     * fonts lack — Bengali, Arabic, Chinese, Thai and every other script the
     * device can display — so `\text{বাংলা}` draws instead of boxes. On by
     * default; turn off to use only fonts you add.
     */
    @Volatile
    var usesSystemFonts: Boolean = true

    /** Memory-mapped font files the engine reads from; kept for its lifetime. */
    private val mappedFonts = ArrayList<java.nio.ByteBuffer>()
    private val triedFonts = HashSet<String>()

    /**
     * Adds a font (any OpenType or TrueType file; [index] picks the face in a
     * collection) for characters the fonts before it lack.
     */
    @Synchronized
    fun addFont(bytes: ByteArray, index: Int = 0) {
        if (NativeBridge.addFont(handle, bytes, index) < 0) throw IllegalArgumentException(NativeBridge.lastError() ?: "bad font")
    }

    /** The characters of [tex] no loaded font can draw ("" when all can). */
    @Synchronized
    fun missingCharacters(tex: String, displayMode: Boolean = true): String =
        NativeBridge.missingChars(handle, tex, displayMode) ?: throw MathParseException(NativeBridge.lastError() ?: "parse failed")

    /** Adds system fonts until [tex] is covered or no system font has the rest. */
    private fun coverWithSystemFonts(tex: String, displayMode: Boolean) {
        if (!usesSystemFonts) return
        repeat(12) {
            val missing = NativeBridge.missingChars(handle, tex, displayMode)
            if (missing.isNullOrEmpty()) return
            val font = SystemFontFinder.find(missing.codePointAt(0), triedFonts) ?: return
            triedFonts.add(font.key)
            val buffer = try {
                java.io.RandomAccessFile(font.file, "r").use { f ->
                    f.channel.map(java.nio.channels.FileChannel.MapMode.READ_ONLY, 0, f.length())
                }
            } catch (e: java.io.IOException) {
                return@repeat
            }
            if (NativeBridge.addFontBuffer(handle, buffer, font.index) >= 0) mappedFonts.add(buffer)
        }
    }

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
        coverWithSystemFonts(tex, displayMode)
        val data = NativeBridge.render(handle, tex, fontSizePx, displayMode, color, macroText, maxWidthPx, hitTesting)
            ?: throw MathParseException(NativeBridge.lastError() ?: "render failed")
        return MathLayout(data)
    }

    /** Lays out an editor, finding system fonts for any script typed into it first. */
    @Synchronized
    internal fun renderEditor(editor: Long, tex: String, fontSizePx: Float, displayMode: Boolean, color: Int): MathLayout {
        coverWithSystemFonts(tex, displayMode)
        val data = NativeBridge.editorRender(editor, handle, fontSizePx, displayMode, color)
            ?: throw MathParseException(NativeBridge.lastError() ?: "render failed")
        return MathLayout(data)
    }

    /**
     * Caps the work one formula may cost on this engine, for input from
     * strangers (a chat, a comment field). Null keeps a limit's default:
     * 256 KB of expanded source, 50,000 nodes, 200,000 drawn items.
     */
    @Synchronized
    fun setBudget(maxExpandedBytes: Long? = null, maxNodes: Long? = null, maxItems: Long? = null) {
        NativeBridge.setBudget(handle, maxExpandedBytes ?: 0, maxNodes ?: 0, maxItems ?: 0)
    }

    /** Layouts kept for repeated requests (default 256); 0 turns caching off. */
    @Synchronized
    fun setCacheCapacity(capacity: Int) {
        NativeBridge.setCacheCapacity(handle, capacity)
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
