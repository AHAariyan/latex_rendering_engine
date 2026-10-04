package dev.mathcore

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.Rect
import android.text.InputType
import android.util.AttributeSet
import android.util.TypedValue
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import android.view.accessibility.AccessibilityEvent
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager

/**
 * An editable formula: a text field for mathematics.
 *
 * It takes the system keyboard and a hardware one (arrows, Shift to select,
 * Ctrl+A/Z/Y/C/X/V), and tells TalkBack the formula and, after each key,
 * where the cursor is.
 *
 *     val field = MathField(context)
 *     field.latex = "x^2"
 *     field.onChange = { tex -> ... }
 */
open class MathField @JvmOverloads constructor(context: Context, attrs: AttributeSet? = null, defStyleAttr: Int = 0) :
    View(context, attrs, defStyleAttr) {

    /** The model, for toolbar buttons: `field.editor.command("frac")` then [refresh]. */
    val editor = MathEditor()

    var engine: MathEngine = MathEngine.shared
        set(value) { field = value; relayout() }

    var textSizePx: Float = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, 20f, resources.displayMetrics)
        set(value) { field = value; relayout() }

    var textColor: Int = Color.BLACK
        set(value) { field = value; relayout() }

    /** The caret and selection colour. */
    var accentColor: Int = 0xFF1A73E8.toInt()
        set(value) { field = value; invalidate() }

    /** Shown, dimmed, while the field is empty and unfocused. */
    var placeholder: String = ""
        set(value) { field = value; invalidate() }

    var isEditable: Boolean = true

    /** The language TalkBack hears (a BCP 47 tag); null follows the device. */
    var speechLanguage: String? = null

    /** Called with the new TeX after every change. */
    var onChange: ((String) -> Unit)? = null

    /** The formula as TeX. */
    var latex: String
        get() = editor.latex
        set(value) { editor.latex = value; relayout() }

    private var layout: MathEditor.Layout? = null
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private var caretOn = true
    private val blink = object : Runnable {
        override fun run() {
            caretOn = !caretOn
            invalidate()
            postDelayed(this, 530)
        }
    }

    init {
        isFocusable = true
        isFocusableInTouchMode = true
        isClickable = true
        val pad = (8 * resources.displayMetrics.density).toInt()
        if (paddingLeft == 0 && paddingTop == 0) setPadding(pad + pad / 4, pad, pad + pad / 4, pad)
        relayout()
    }

    // ---- layout and drawing ----

    private fun relayout() {
        layout = try {
            editor.layout(engine, textSizePx, true, textColor)
        } catch (e: MathParseException) {
            null
        }
        contentDescription = editor.speech(speechLanguage).ifEmpty { placeholder.ifEmpty { "math" } }
        requestLayout()
        invalidate()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val w = (layout?.formula?.width ?: 0f).coerceAtLeast(textSizePx * 2) + paddingLeft + paddingRight
        val h = (layout?.formula?.height ?: 0f).coerceAtLeast(textSizePx * 1.2f) + paddingTop + paddingBottom
        setMeasuredDimension(resolveSize(w.toInt() + 1, widthMeasureSpec), resolveSize(h.toInt() + 1, heightMeasureSpec))
    }

    /** Where the formula's top-left corner is drawn. */
    private val originX: Float get() = paddingLeft.toFloat()
    private val originY: Float
        get() = maxOf(paddingTop.toFloat(), (height - (layout?.formula?.height ?: 0f)) / 2)

    override fun onDraw(canvas: Canvas) {
        val l = layout ?: return
        paint.style = Paint.Style.FILL
        paint.color = (accentColor and 0x00FFFFFF) or 0x4D000000
        for (r in l.selection) canvas.drawRect(originX + r.left, originY + r.top, originX + r.right, originY + r.bottom, paint)
        if (editor.latex.isEmpty() && placeholder.isNotEmpty() && !isFocused) {
            paint.color = (textColor and 0x00FFFFFF) or 0x73000000
            paint.textSize = textSizePx * 0.8f
            canvas.drawText(placeholder, originX, originY + textSizePx * 0.8f, paint)
        } else {
            engine.draw(l.formula, canvas, originX, originY)
        }
        if (isFocused && isEditable && caretOn) {
            paint.color = accentColor
            val w = maxOf(l.caret.width(), 2f * resources.displayMetrics.density)
            canvas.drawRect(originX + l.caret.left, originY + l.caret.top, originX + l.caret.left + w, originY + l.caret.bottom, paint)
        }
    }

    override fun onFocusChanged(gainFocus: Boolean, direction: Int, previouslyFocusedRect: Rect?) {
        super.onFocusChanged(gainFocus, direction, previouslyFocusedRect)
        removeCallbacks(blink)
        caretOn = true
        if (gainFocus) postDelayed(blink, 530)
        invalidate()
    }

    override fun onDetachedFromWindow() {
        removeCallbacks(blink)
        super.onDetachedFromWindow()
    }

    // ---- editing ----

    /** Redraws after the model was changed from outside (a toolbar button). */
    fun refresh(changed: Boolean = true) {
        relayout()
        caretOn = true
        if (changed) onChange?.invoke(editor.latex)
        if (isAccessibilityFocused || isFocused) announceForAccessibility(editor.cursorDescription(speechLanguage))
        if (changed) sendAccessibilityEvent(AccessibilityEvent.TYPE_VIEW_TEXT_CHANGED)
    }

    private inline fun change(edit: () -> Unit) {
        if (!isEditable) return
        val before = editor.latex
        edit()
        refresh(editor.latex != before)
    }

    private inline fun move(action: () -> Unit) {
        action()
        refresh(false)
    }

    /** Types [text] as the keyboard would, for on-screen keys. */
    fun type(text: String) = change { editor.type(text) }

    /** Runs an editor command (`frac`, `sqrt`, `alpha`...), for toolbar buttons. */
    fun command(name: String) = change { editor.command(name) }

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (event.actionMasked == MotionEvent.ACTION_UP) {
            requestFocus()
            if (isEditable) {
                (context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager).showSoftInput(this, 0)
            }
            move { editor.tap(event.x - originX, event.y - originY) }
            performClick()
        }
        return true
    }

    override fun onCheckIsTextEditor(): Boolean = isEditable

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection {
        // No suggestions and no composing text: every character arrives at once.
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS or
            InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
        outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI or EditorInfo.IME_FLAG_NO_FULLSCREEN or EditorInfo.IME_ACTION_DONE
        return object : BaseInputConnection(this, false) {
            override fun commitText(text: CharSequence, newCursorPosition: Int): Boolean {
                if (text == "\n") move { editor.key(MathEditor.Key.Enter) } else change { editor.type(text.toString()) }
                return true
            }

            override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
                repeat(beforeLength.coerceAtLeast(1)) { change { editor.key(MathEditor.Key.Backspace) } }
                return true
            }

            override fun performEditorAction(actionCode: Int): Boolean {
                move { editor.key(MathEditor.Key.Enter) }
                return true
            }
        }
    }

    override fun onKeyDown(keyCode: Int, event: KeyEvent): Boolean {
        val shift = event.isShiftPressed
        if (event.isCtrlPressed) {
            when (keyCode) {
                KeyEvent.KEYCODE_A -> move { editor.selectAll() }
                KeyEvent.KEYCODE_Z -> change { if (shift) editor.redo() else editor.undo() }
                KeyEvent.KEYCODE_Y -> change { editor.redo() }
                KeyEvent.KEYCODE_C -> copy()
                KeyEvent.KEYCODE_X -> { copy(); change { if (editor.selectedLatex.isNotEmpty()) editor.key(MathEditor.Key.Backspace) } }
                KeyEvent.KEYCODE_V -> paste()
                else -> return super.onKeyDown(keyCode, event)
            }
            return true
        }
        when (keyCode) {
            KeyEvent.KEYCODE_DPAD_LEFT -> move { editor.key(MathEditor.Key.Left, shift) }
            KeyEvent.KEYCODE_DPAD_RIGHT -> move { editor.key(MathEditor.Key.Right, shift) }
            KeyEvent.KEYCODE_DPAD_UP -> move { editor.key(MathEditor.Key.Up) }
            KeyEvent.KEYCODE_DPAD_DOWN -> move { editor.key(MathEditor.Key.Down) }
            KeyEvent.KEYCODE_MOVE_HOME -> move { editor.key(MathEditor.Key.Home) }
            KeyEvent.KEYCODE_MOVE_END -> move { editor.key(MathEditor.Key.End) }
            KeyEvent.KEYCODE_DEL -> change { editor.key(MathEditor.Key.Backspace) }
            KeyEvent.KEYCODE_FORWARD_DEL -> change { editor.key(MathEditor.Key.Delete) }
            KeyEvent.KEYCODE_ENTER, KeyEvent.KEYCODE_NUMPAD_ENTER -> move { editor.key(MathEditor.Key.Enter) }
            else -> {
                val c = event.unicodeChar
                if (c == 0 || Character.isISOControl(c)) return super.onKeyDown(keyCode, event)
                change { editor.type(String(Character.toChars(c))) }
            }
        }
        return true
    }

    /** Copies the selection, or the whole formula without one, as TeX. */
    fun copy() {
        val tex = editor.selectedLatex.ifEmpty { editor.latex }
        (context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).setPrimaryClip(ClipData.newPlainText("TeX", tex))
    }

    /** Pastes TeX from the clipboard as structure. */
    fun paste() {
        val clip = (context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager).primaryClip ?: return
        if (clip.itemCount == 0) return
        val text = clip.getItemAt(0).coerceToText(context)?.toString().orEmpty()
        if (text.isNotEmpty()) change { editor.insertLatex(text) }
    }

    override fun getAccessibilityClassName(): CharSequence = "android.widget.EditText"
}
