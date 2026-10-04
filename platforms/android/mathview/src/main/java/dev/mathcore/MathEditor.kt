package dev.mathcore

import android.graphics.Color
import android.graphics.RectF
import java.io.Closeable

/**
 * The model behind a math input field: forward text, keys and taps to it,
 * lay it out, and draw the formula with the caret and selection it reports.
 * [MathField] is the ready-made view; use this directly to build your own.
 *
 * Typing follows the usual conventions: `/` makes a fraction of the term
 * before it, `^` and `_` open a script, `(` opens a pair, `\` starts a
 * command name, and `sqrt`, `pi`, `sin`... become what they name.
 */
class MathEditor(latex: String = "") : Closeable {
    private var handle: Long = NativeBridge.editorCreate().also { if (latex.isNotEmpty()) NativeBridge.editorSetTex(it, latex) }

    /** The content as TeX. Setting it puts the cursor at the end. */
    var latex: String
        @Synchronized get() = NativeBridge.editorTex(handle) ?: ""
        @Synchronized set(value) = NativeBridge.editorSetTex(handle, value)

    /** The selection as TeX, empty without one. */
    val selectedLatex: String
        @Synchronized get() = NativeBridge.editorSelectedTex(handle) ?: ""

    /** Types text at the cursor. */
    @Synchronized
    fun type(text: String) = NativeBridge.editorInput(handle, 0, text)

    /** Inserts TeX as structure (a pasted `\frac{a}{b}` stays a fraction). */
    @Synchronized
    fun insertLatex(latex: String) = NativeBridge.editorInput(handle, 1, latex)

    /** Runs a command by name: `frac`, `sqrt`, `nthroot`, `alpha`... */
    @Synchronized
    fun command(name: String) = NativeBridge.editorInput(handle, 2, name)

    /** A key the editor handles. */
    enum class Key(internal val code: String) {
        Left("ArrowLeft"), Right("ArrowRight"), Up("ArrowUp"), Down("ArrowDown"),
        Home("Home"), End("End"), Backspace("Backspace"), Delete("Delete"), Enter("Enter"),
    }

    /** Sends a key; with [shift], arrows extend the selection. */
    @Synchronized
    fun key(key: Key, shift: Boolean = false) {
        NativeBridge.editorKey(handle, key.code, shift, false)
    }

    @Synchronized fun selectAll() { NativeBridge.editorKey(handle, "a", false, true) }
    @Synchronized fun undo() { NativeBridge.editorKey(handle, "z", false, true) }
    @Synchronized fun redo() { NativeBridge.editorKey(handle, "z", true, true) }

    /** The editor drawn: the formula, the caret and the selection. */
    class Layout(val formula: MathLayout, val caret: RectF, val selection: List<RectF>)

    /**
     * Lays the editor out. Draw [Layout.formula] with [MathEngine.draw], then
     * the selection and the caret; [tap] refers to this layout.
     */
    @Synchronized
    fun layout(engine: MathEngine = MathEngine.shared, fontSizePx: Float, displayMode: Boolean = true, color: Int = Color.BLACK): Layout {
        val formula = engine.renderEditor(handle, NativeBridge.editorTex(handle) ?: "", fontSizePx, displayMode, color)
        val f = NativeBridge.editorCaret(handle) ?: FloatArray(4)
        fun rect(i: Int) = RectF(f[i], f[i + 1], f[i] + f[i + 2], f[i + 1] + f[i + 3])
        return Layout(formula, rect(0), (4 until f.size step 4).map(::rect))
    }

    /** Moves the cursor to a point of the last layout. */
    @Synchronized
    fun tap(x: Float, y: Float) = NativeBridge.editorTap(handle, x, y)

    /**
     * What a screen reader says for the cursor's place ("denominator, 2"),
     * in [language] (a BCP 47 tag; the device's when null).
     */
    @Synchronized
    fun cursorDescription(language: String? = null): String =
        NativeBridge.editorSpeech(handle, false, language ?: MathAccessibility.deviceLanguage()) ?: ""

    /** The whole formula read aloud. */
    @Synchronized
    fun speech(language: String? = null): String =
        NativeBridge.editorSpeech(handle, true, language ?: MathAccessibility.deviceLanguage()) ?: ""

    @Synchronized
    override fun close() {
        if (handle != 0L) {
            NativeBridge.editorDestroy(handle)
            handle = 0L
        }
    }
}
