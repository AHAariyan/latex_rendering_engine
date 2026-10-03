package dev.mathcore.reactnative

import com.facebook.react.bridge.ReactApplicationContext
import dev.mathcore.MathAccessibility
import dev.mathcore.MathEngine
import dev.mathcore.SpeechVerbosity

/** Synchronous calls into the engine; none needs a view. */
class MathCoreModule(context: ReactApplicationContext) : NativeMathCoreSpec(context) {
    override fun getName() = NAME

    override fun speech(tex: String, verbosity: Double): String =
        MathAccessibility.speech(tex, level(verbosity))

    // The engine's JSON, untouched: JavaScript parses it.
    override fun speechTree(tex: String, verbosity: Double): String =
        MathAccessibility.speechTreeJson(tex, level(verbosity))

    override fun mathml(tex: String, displayMode: Boolean): String = MathAccessibility.mathml(tex, displayMode)

    override fun asciimathToTex(source: String): String = MathEngine.asciimathToTex(source)

    override fun nemeth(tex: String): String = MathAccessibility.nemeth(tex)

    private fun level(v: Double) = SpeechVerbosity.entries.getOrElse(v.toInt()) { SpeechVerbosity.Brief }

    companion object {
        const val NAME = "MathCore"
    }
}
