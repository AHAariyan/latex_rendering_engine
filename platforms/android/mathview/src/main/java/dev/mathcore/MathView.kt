package dev.mathcore

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.util.AttributeSet
import android.util.TypedValue
import android.view.View

/**
 * A View that typesets a TeX formula natively. Set [latex], [textSizePx],
 * [textColor] and [displayMode]; the view measures itself to the formula.
 *
 * With [wrap] on (the default) a formula too wide for the width the parent
 * offers is broken into lines before relations and binary operators.
 */
class MathView @JvmOverloads constructor(context: Context, attrs: AttributeSet? = null, defStyleAttr: Int = 0) :
    View(context, attrs, defStyleAttr) {

    var engine: MathEngine = MathEngine.shared
        set(value) { field = value; relayout() }

    var latex: String = ""
        set(value) { if (field != value) { field = value; relayout() } }

    var textSizePx: Float = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_SP, 18f, resources.displayMetrics)
        set(value) { field = value; relayout() }

    var textColor: Int = Color.BLACK
        set(value) { field = value; relayout() }

    var displayMode: Boolean = true
        set(value) { field = value; relayout() }

    var wrap: Boolean = true
        set(value) { field = value; relayout() }

    /** Non-null when the current [latex] failed to parse. */
    var error: String? = null
        private set

    private var layoutResult: MathLayout? = null
    private var appliedWidth: Float = -1f

    /** Recomputes the layout. Safe to call during measurement. */
    private fun compute(maxWidthPx: Float) {
        appliedWidth = maxWidthPx
        layoutResult = try {
            error = null
            if (latex.isBlank()) null else engine.render(latex, textSizePx, displayMode, textColor, emptyMap(), maxWidthPx)
        } catch (e: MathParseException) {
            error = e.message
            null
        }
    }

    private fun relayout() {
        compute(appliedWidth.coerceAtLeast(0f))
        requestLayout()
        invalidate()
    }

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val mode = MeasureSpec.getMode(widthMeasureSpec)
        val available = (MeasureSpec.getSize(widthMeasureSpec) - paddingLeft - paddingRight).toFloat()
        val wanted = if (wrap && mode != MeasureSpec.UNSPECIFIED && available > 0f) available else 0f
        if (wanted != appliedWidth || layoutResult == null && error == null) {
            compute(wanted)
        }
        val l = layoutResult
        val w = (l?.width ?: 0f) + paddingLeft + paddingRight
        val h = (l?.height ?: 0f) + paddingTop + paddingBottom
        setMeasuredDimension(
            resolveSize(Math.ceil(w.toDouble()).toInt(), widthMeasureSpec),
            resolveSize(Math.ceil(h.toDouble()).toInt(), heightMeasureSpec),
        )
    }

    override fun onDraw(canvas: Canvas) {
        val l = layoutResult ?: return
        engine.draw(l, canvas, paddingLeft.toFloat(), paddingTop.toFloat())
    }

    /** Baseline of the first line, for alignment with surrounding text. */
    override fun getBaseline(): Int = layoutResult?.let { (it.ascent + paddingTop).toInt() } ?: super.getBaseline()

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        layoutResult = null
    }
}
