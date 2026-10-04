package dev.mathcore.reactnative

import android.content.Context
import android.graphics.Color
import android.util.TypedValue
import android.view.inputmethod.InputMethodManager
import com.facebook.react.bridge.Arguments
import com.facebook.react.bridge.ReactContext
import com.facebook.react.bridge.WritableMap
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.UIManagerHelper
import com.facebook.react.uimanager.ViewManagerDelegate
import com.facebook.react.uimanager.events.Event
import com.facebook.react.viewmanagers.MathCoreFieldManagerDelegate
import com.facebook.react.viewmanagers.MathCoreFieldManagerInterface
import dev.mathcore.MathField

/** The Fabric view for <MathField>: dev.mathcore.MathField, reporting changes and its size. */
@ReactModule(name = MathCoreFieldManager.NAME)
class MathCoreFieldManager : SimpleViewManager<RNMathField>(), MathCoreFieldManagerInterface<RNMathField> {
    private val delegate = MathCoreFieldManagerDelegate(this)

    override fun getDelegate(): ViewManagerDelegate<RNMathField> = delegate
    override fun getName() = NAME
    override fun createViewInstance(context: ThemedReactContext) = RNMathField(context)

    override fun setValue(view: RNMathField, value: String?) { view.pendingValue = value }
    override fun setFontSize(view: RNMathField, value: Float) { view.fontSizeDp = if (value > 0) value else 20f }
    override fun setColor(view: RNMathField, value: Int?) { view.textColor = value ?: Color.BLACK }
    override fun setCursorColor(view: RNMathField, value: Int?) { view.accentColor = value ?: 0xFF1A73E8.toInt() }
    override fun setPlaceholder(view: RNMathField, value: String?) { view.placeholder = value ?: "" }
    override fun setEditable(view: RNMathField, value: Boolean) { view.isEditable = value }

    override fun runCommand(view: RNMathField, name: String) { view.command(name) }
    override fun typeText(view: RNMathField, text: String) { view.type(text) }
    override fun focus(view: RNMathField) { view.requestFocus(); view.showKeyboard() }
    override fun blur(view: RNMathField) { view.clearFocus(); view.hideKeyboard() }

    override fun onAfterUpdateTransaction(view: RNMathField) {
        super.onAfterUpdateTransaction(view)
        view.commit()
    }

    override fun getExportedCustomDirectEventTypeConstants(): Map<String, Any> = mapOf(
        "topMathChange" to mapOf("registrationName" to "onMathChange"),
        "topMathSize" to mapOf("registrationName" to "onMathSize"),
    )

    companion object {
        const val NAME = "MathCoreField"
    }
}

/** MathField with props applied in one batch, its changes and size sent to JavaScript. */
class RNMathField(context: ThemedReactContext) : MathField(context) {
    var pendingValue: String? = null
    var fontSizeDp = 20f
    private var reported = Pair(-1f, -1f)

    init {
        onChange = { tex ->
            emit("topMathChange", Arguments.createMap().apply { putString("latex", tex) })
            report()
        }
    }

    fun commit() {
        textSizePx = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, fontSizeDp, resources.displayMetrics)
        // The value JavaScript echoes back after a change is the one held: leave the cursor alone.
        pendingValue?.let { if (it != latex) latex = it }
        pendingValue = null
        report()
    }

    fun showKeyboard() {
        (context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager).showSoftInput(this, 0)
    }

    fun hideKeyboard() {
        (context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager).hideSoftInputFromWindow(windowToken, 0)
    }

    private fun report() {
        val spec = MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED)
        measure(spec, spec)
        val density = resources.displayMetrics.density
        val size = Pair(measuredWidth / density, measuredHeight / density)
        if (size == reported) return
        reported = size
        emit("topMathSize", Arguments.createMap().apply { putDouble("width", size.first.toDouble()); putDouble("height", size.second.toDouble()) })
    }

    // The tag form is deprecated from 0.82 but is the one 0.80 and 0.81 have.
    @Suppress("DEPRECATION")
    private fun emit(name: String, data: WritableMap) {
        val ctx = context as ReactContext
        val surface = UIManagerHelper.getSurfaceId(this)
        UIManagerHelper.getEventDispatcherForReactTag(ctx, id)?.dispatchEvent(FieldEvent(surface, id, name, data))
    }
}

private class FieldEvent(surfaceId: Int, viewId: Int, private val name: String, private val data: WritableMap) :
    Event<FieldEvent>(surfaceId, viewId) {
    override fun getEventName() = name
    override fun getEventData() = data
}
