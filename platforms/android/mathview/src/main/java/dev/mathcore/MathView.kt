package dev.mathcore

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Rect
import android.os.Bundle
import android.util.AttributeSet
import android.util.TypedValue
import android.view.KeyEvent
import android.view.MotionEvent
import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.accessibility.AccessibilityNodeInfoCompat
import androidx.customview.widget.ExploreByTouchHelper

/**
 * A View that typesets a TeX formula natively. Set [latex], [textSizePx],
 * [textColor] and [displayMode]; the view measures itself to the formula.
 *
 * With [wrap] on (the default) a formula too wide for the width the parent
 * offers is broken into lines before relations and binary operators.
 *
 * TalkBack first reads the whole formula, then lets the user swipe through its
 * parts (each term, fraction, script or matrix row) with the part outlined, or
 * touch a part to hear it.
 */
open class MathView @JvmOverloads constructor(context: Context, attrs: AttributeSet? = null, defStyleAttr: Int = 0) :
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

    /** How much scaffolding TalkBack hears. */
    var speechVerbosity: SpeechVerbosity = SpeechVerbosity.Brief
        set(value) { field = value; relayout() }

    /** The language TalkBack hears the formula in (a BCP 47 tag); null follows the device. */
    var speechLanguage: String? = null
        set(value) { field = value; relayout() }

    /** Source regions are always recorded now; kept for compatibility. */
    @Deprecated("Regions are always recorded; regionAt works without it.")
    var hitTesting: Boolean = true

    /** Called with the smallest sub-expression under a tap. */
    var onTap: ((MathRegion) -> Unit)? = null
        set(value) {
            field = value
            isClickable = value != null
        }

    /** Non-null when the current [latex] failed to parse. */
    var error: String? = null
        private set

    private var layoutResult: MathLayout? = null
    private var parts: List<SpeechNode> = emptyList()
    private var appliedWidth: Float = -1f
    private val explorer = Explorer()

    init {
        ViewCompat.setAccessibilityDelegate(this, explorer)
    }

    /** Recomputes the layout. Safe to call during measurement. */
    private fun compute(maxWidthPx: Float) {
        appliedWidth = maxWidthPx
        layoutResult = try {
            error = null
            if (latex.isBlank()) {
                null
            } else {
                engine.render(latex, textSizePx, displayMode, textColor, emptyMap(), maxWidthPx, hitTesting = true)
            }
        } catch (e: MathParseException) {
            error = e.message
            null
        }
        val language = speechLanguage ?: MathAccessibility.deviceLanguage()
        contentDescription = MathAccessibility.speechOrNull(latex, speechVerbosity, language) ?: latex
        val tree = if (layoutResult == null) null else MathAccessibility.speechTreeOrNull(latex, speechVerbosity, language)
        parts = tree?.children?.takeIf { it.size > 1 } ?: emptyList()
        explorer.invalidateRoot()
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

    override fun onTouchEvent(event: MotionEvent): Boolean {
        val tap = onTap ?: return super.onTouchEvent(event)
        if (event.actionMasked == MotionEvent.ACTION_UP) {
            regionAt(event.x, event.y)?.let(tap)
            performClick()
        }
        return true
    }

    override fun performClick(): Boolean = super.performClick()

    override fun dispatchHoverEvent(event: MotionEvent): Boolean =
        explorer.dispatchHoverEvent(event) || super.dispatchHoverEvent(event)

    override fun dispatchKeyEvent(event: KeyEvent): Boolean =
        explorer.dispatchKeyEvent(event) || super.dispatchKeyEvent(event)

    override fun onFocusChanged(gainFocus: Boolean, direction: Int, previouslyFocusedRect: Rect?) {
        super.onFocusChanged(gainFocus, direction, previouslyFocusedRect)
        explorer.onFocusChanged(gainFocus, direction, previouslyFocusedRect)
    }

    /** The smallest sub-expression under a point in this view's coordinates; indexes into [latex]. */
    fun regionAt(x: Float, y: Float): MathRegion? =
        layoutResult?.hitNearest(x - paddingLeft, y - paddingTop)

    /** Baseline of the first line, for alignment with surrounding text. */
    override fun getBaseline(): Int = layoutResult?.let { (it.ascent + paddingTop).toInt() } ?: super.getBaseline()

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        layoutResult = null
    }

    /** Exposes the formula's top-level parts to TalkBack as virtual views. */
    private inner class Explorer : ExploreByTouchHelper(this@MathView) {
        private fun bounds(part: SpeechNode): Rect? {
            val regions = layoutResult?.highlight(part.start, part.end).orEmpty()
            if (regions.isEmpty()) return null
            val r = Rect(
                regions.minOf { it.x }.toInt(),
                regions.minOf { it.y }.toInt(),
                regions.maxOf { it.x + it.width }.toInt() + 1,
                regions.maxOf { it.y + it.height }.toInt() + 1,
            )
            r.offset(paddingLeft, paddingTop)
            return r
        }

        override fun getVirtualViewAt(x: Float, y: Float): Int {
            val i = parts.indices.firstOrNull { bounds(parts[it])?.contains(x.toInt(), y.toInt()) == true }
            return i ?: INVALID_ID
        }

        override fun getVisibleVirtualViews(ids: MutableList<Int>) {
            parts.indices.filterTo(ids) { bounds(parts[it]) != null }
        }

        // ExploreByTouchHelper still requires bounds in the parent's coordinates.
        @Suppress("DEPRECATION")
        override fun onPopulateNodeForVirtualView(id: Int, node: AccessibilityNodeInfoCompat) {
            val part = parts.getOrNull(id)
            node.contentDescription = part?.announcement ?: ""
            node.setBoundsInParent(part?.let(::bounds) ?: Rect(0, 0, 1, 1))
        }

        override fun onPerformActionForVirtualView(id: Int, action: Int, arguments: Bundle?): Boolean = false
    }
}
