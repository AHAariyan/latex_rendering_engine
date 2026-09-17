package dev.mathcore

/**
 * Accessibility output for a formula. Neither call needs an engine or a font;
 * both read the parsed formula, so what a screen reader announces is what the
 * engine drew.
 */
object MathAccessibility {
    /** Presentation MathML, for assistive technology that consumes it. */
    fun mathml(latex: String, displayMode: Boolean = true): String =
        NativeBridge.mathml(latex, displayMode) ?: throw MathParseException(NativeBridge.lastError() ?: "mathml failed")

    /**
     * A spoken sentence for TalkBack, suitable for a `contentDescription`.
     * `x^2 + y^2 = z^2` becomes "x squared plus y squared equals z squared".
     */
    fun speech(latex: String): String =
        NativeBridge.speech(latex) ?: throw MathParseException(NativeBridge.lastError() ?: "speech failed")

    /** [speech] for a formula that may not parse; null when it does not. */
    fun speechOrNull(latex: String): String? = try {
        speech(latex)
    } catch (_: MathParseException) {
        null
    }
}
