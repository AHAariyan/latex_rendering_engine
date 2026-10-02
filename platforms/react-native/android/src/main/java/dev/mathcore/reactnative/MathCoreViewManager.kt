package dev.mathcore.reactnative

import android.graphics.Color
import android.util.TypedValue
import com.facebook.react.bridge.Arguments
import com.facebook.react.bridge.ReactContext
import com.facebook.react.bridge.WritableMap
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.UIManagerHelper
import com.facebook.react.uimanager.ViewManagerDelegate
import com.facebook.react.uimanager.events.Event
import com.facebook.react.viewmanagers.MathCoreViewManagerDelegate
import com.facebook.react.viewmanagers.MathCoreViewManagerInterface
import dev.mathcore.MathEngine
import dev.mathcore.MathParseException
import dev.mathcore.MathView
import dev.mathcore.SpeechVerbosity

/** The Fabric view: dev.mathcore.MathView, reporting its content size to JavaScript. */
@ReactModule(name = MathCoreViewManager.NAME)
class MathCoreViewManager : SimpleViewManager<RNMathView>(), MathCoreViewManagerInterface<RNMathView> {
    private val delegate = MathCoreViewManagerDelegate(this)

    override fun getDelegate(): ViewManagerDelegate<RNMathView> = delegate
    override fun getName() = NAME
    override fun createViewInstance(context: ThemedReactContext) = RNMathView(context)

    override fun setLatex(view: RNMathView, value: String?) { view.source = value ?: "" }
    override fun setFontSize(view: RNMathView, value: Float) { view.fontSizeDp = value }
    override fun setColor(view: RNMathView, value: Int?) { view.textColor = value ?: Color.BLACK }
    override fun setDisplayMode(view: RNMathView, value: Boolean) { view.displayMode = value }
    override fun setWrap(view: RNMathView, value: Boolean) { view.wrap = value }
    override fun setAsciimath(view: RNMathView, value: Boolean) { view.asciimath = value }
    override fun setSpeechVerbosity(view: RNMathView, value: Int) {
        view.speechVerbosity = SpeechVerbosity.entries.getOrElse(value) { SpeechVerbosity.Brief }
    }

    override fun onAfterUpdateTransaction(view: RNMathView) {
        super.onAfterUpdateTransaction(view)
        view.commit()
    }

    override fun getExportedCustomDirectEventTypeConstants(): Map<String, Any> = mapOf(
        "topMathSize" to mapOf("registrationName" to "onMathSize"),
        "topMathTap" to mapOf("registrationName" to "onMathTap"),
        "topMathError" to mapOf("registrationName" to "onMathError"),
    )

    companion object {
        const val NAME = "MathCoreView"
    }
}

/** MathView with props applied in one batch and its size reported in dp. */
class RNMathView(context: ThemedReactContext) : MathView(context) {
    var source = ""
    var fontSizeDp = 17f
    var asciimath = false
    private var reported = Pair(-1f, -1f)
    private var reportedError: String? = null

    init {
        onTap = { region -> emit("topMathTap", Arguments.createMap().apply { putInt("start", region.start); putInt("end", region.end) }) }
    }

    fun commit() {
        val density = resources.displayMetrics.density
        textSizePx = TypedValue.applyDimension(TypedValue.COMPLEX_UNIT_DIP, fontSizeDp, resources.displayMetrics)
        latex = try {
            if (asciimath) MathEngine.asciimathToTex(source) else source
        } catch (e: MathParseException) {
            source
        }
        if (error != reportedError) {
            reportedError = error
            error?.let { emit("topMathError", Arguments.createMap().apply { putString("message", it) }) }
        }
        report(density)
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        // A new width re-breaks the formula; its height follows.
        if (w != oldw) {
            measure(MeasureSpec.makeMeasureSpec(w, if (wrap) MeasureSpec.AT_MOST else MeasureSpec.UNSPECIFIED), MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED))
            report(resources.displayMetrics.density)
        }
    }

    private fun report(density: Float) {
        if (width == 0 || !wrap) {
            measure(MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED), MeasureSpec.makeMeasureSpec(0, MeasureSpec.UNSPECIFIED))
        }
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
        UIManagerHelper.getEventDispatcherForReactTag(ctx, id)?.dispatchEvent(MathEvent(surface, id, name, data))
    }
}

private class MathEvent(surfaceId: Int, viewId: Int, private val name: String, private val data: WritableMap) :
    Event<MathEvent>(surfaceId, viewId) {
    override fun getEventName() = name
    override fun getEventData() = data
}
