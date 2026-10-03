package dev.mathcore

import java.util.Locale

/** How much scaffolding spoken math carries. */
enum class SpeechVerbosity(internal val level: Int) {
    /** Every structure is opened and closed: "the fraction 1 over 2, end fraction". */
    Verbose(0),

    /** Scaffolding only where the reading would be ambiguous: "1 over 2". The default. */
    Brief(1),

    /** Content words only. */
    Superbrief(2),
}

/**
 * One part of a formula for a screen reader to step through. [start] and
 * [end] are the byte range of the source, for [MathLayout.highlight].
 */
data class SpeechNode(
    /** "formula", "fraction", "root", "scripts", "matrix", "symbol", ... */
    val role: String,
    /** Place in the parent: "numerator", "superscript", "row 2", or empty. */
    val label: String,
    val text: String,
    val start: Int,
    val end: Int,
    val children: List<SpeechNode>,
) {
    /** What TalkBack says for this part on its own: its place, then its content. */
    val announcement: String get() = if (label.isEmpty()) text else "$label: $text"
}

/**
 * Accessibility output for a formula. None of these calls needs an engine or a
 * font; all read the parsed formula, so what a screen reader announces is what
 * the engine drew.
 */
object MathAccessibility {
    /**
     * Languages spoken math is available in, as BCP 47 tags. Any other
     * language reads in English.
     */
    val languages: List<String> = listOf("en", "es", "fr", "de", "pt", "bn", "hi")

    /** The device's language, which speech follows unless told otherwise. */
    fun deviceLanguage(): String = Locale.getDefault().toLanguageTag()

    /** Presentation MathML, for assistive technology that consumes it. */
    fun mathml(latex: String, displayMode: Boolean = true): String =
        NativeBridge.mathml(latex, displayMode) ?: throw MathParseException(NativeBridge.lastError() ?: "mathml failed")

    /**
     * A spoken sentence for TalkBack, suitable for a `contentDescription`.
     * `x^2 + y^2 = z^2` becomes "x squared plus y squared equals z squared".
     */
    fun speech(
        latex: String,
        verbosity: SpeechVerbosity = SpeechVerbosity.Brief,
        language: String = deviceLanguage(),
    ): String =
        NativeBridge.speechWith(latex, verbosity.level, language)
            ?: throw MathParseException(NativeBridge.lastError() ?: "speech failed")

    /** [speech] for a formula that may not parse; null when it does not. */
    fun speechOrNull(
        latex: String,
        verbosity: SpeechVerbosity = SpeechVerbosity.Brief,
        language: String = deviceLanguage(),
    ): String? = try {
        speech(latex, verbosity, language)
    } catch (_: MathParseException) {
        null
    }

    /** The formula as a tree a screen reader can walk part by part. */
    fun speechTree(
        latex: String,
        verbosity: SpeechVerbosity = SpeechVerbosity.Brief,
        language: String = deviceLanguage(),
    ): SpeechNode {
        val json = NativeBridge.speechTree(latex, verbosity.level, language)
            ?: throw MathParseException(NativeBridge.lastError() ?: "speech failed")
        return SpeechJson(json).node()
    }

    /** The formula in Nemeth braille (Unicode braille cells), for braille displays. */
    fun nemeth(latex: String): String =
        NativeBridge.nemeth(latex) ?: throw MathParseException(NativeBridge.lastError() ?: "braille failed")

    /** [speechTree] as the engine's JSON, for hosts that pass it on (React Native, a WebView). */
    fun speechTreeJson(
        latex: String,
        verbosity: SpeechVerbosity = SpeechVerbosity.Brief,
        language: String = deviceLanguage(),
    ): String =
        NativeBridge.speechTree(latex, verbosity.level, language)
            ?: throw MathParseException(NativeBridge.lastError() ?: "speech failed")

    /** [speechTree] for a formula that may not parse. */
    fun speechTreeOrNull(
        latex: String,
        verbosity: SpeechVerbosity = SpeechVerbosity.Brief,
        language: String = deviceLanguage(),
    ): SpeechNode? = try {
        speechTree(latex, verbosity, language)
    } catch (_: MathParseException) {
        null
    }
}

/**
 * Reads the speech tree's JSON. The shape is fixed by the engine, so a small
 * reader beats a dependency, and it runs in plain JVM unit tests where
 * Android's org.json is a stub.
 */
internal class SpeechJson(private val s: String) {
    private var i = 0

    fun node(): SpeechNode {
        var role = ""
        var label = ""
        var text = ""
        var start = 0
        var end = 0
        var children = emptyList<SpeechNode>()
        expect('{')
        while (true) {
            skipWs()
            if (peek() == '}') { i++; break }
            val key = string()
            expect(':')
            when (key) {
                "role" -> role = string()
                "label" -> label = string()
                "text" -> text = string()
                "start" -> start = number()
                "end" -> end = number()
                "children" -> children = array()
                else -> throw IllegalStateException("unexpected key $key")
            }
            skipWs()
            if (peek() == ',') i++
        }
        return SpeechNode(role, label, text, start, end, children)
    }

    private fun array(): List<SpeechNode> {
        expect('[')
        val out = ArrayList<SpeechNode>()
        while (true) {
            skipWs()
            if (peek() == ']') { i++; return out }
            out.add(node())
            skipWs()
            if (peek() == ',') i++
        }
    }

    private fun string(): String {
        expect('"')
        val b = StringBuilder()
        while (true) {
            val c = s[i++]
            when (c) {
                '"' -> return b.toString()
                '\\' -> when (val e = s[i++]) {
                    'n' -> b.append('\n')
                    't' -> b.append('\t')
                    'r' -> b.append('\r')
                    'u' -> { b.append(s.substring(i, i + 4).toInt(16).toChar()); i += 4 }
                    else -> b.append(e)
                }
                else -> b.append(c)
            }
        }
    }

    private fun number(): Int {
        skipWs()
        val from = i
        while (i < s.length && (s[i].isDigit() || s[i] == '-')) i++
        return s.substring(from, i).toInt()
    }

    private fun expect(c: Char) {
        skipWs()
        check(s[i] == c) { "expected '$c' at $i" }
        i++
    }

    private fun peek(): Char = s[i]

    private fun skipWs() {
        while (i < s.length && s[i].isWhitespace()) i++
    }
}
