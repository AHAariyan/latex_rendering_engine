package dev.mathcore

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** On a device: text in other scripts draws with the device's own fonts. */
@RunWith(AndroidJUnit4::class)
class SystemFontsTest {
    private val samples = mapOf(
        "bn" to "বাংলা গণিত",
        "hi" to "हिन्दी",
        "ar" to "مساحة الدائرة",
        "he" to "שלום",
        "zh" to "圆的面积",
        "ja" to "円の面積",
        "ko" to "원의 넓이",
        "th" to "พื้นที่",
        "ta" to "பரப்பளவு",
        "ru" to "площадь",
        "el" to "εμβαδόν",
    )

    @Test
    fun everyScriptFindsAFont() {
        for ((lang, text) in samples) {
            val tex = "x = \\text{$text}"
            val engine = MathEngine.bundled()
            engine.usesSystemFonts = false
            assertTrue("$lang: the bundled fonts alone lack it", engine.missingCharacters(tex).isNotEmpty())
            engine.usesSystemFonts = true
            val layout = engine.render(tex, 40f)
            assertEquals("$lang: still missing", "", engine.missingCharacters(tex))
            var fromSystem = false
            layout.forEach(glyph = { font, _, _, _, _, _ -> if (font > 1) fromSystem = true }, rule = { _, _, _, _, _ -> }, line = { _, _, _, _, _, _ -> })
            assertTrue("$lang: drawn from a system font", fromSystem)
            engine.close()
        }
    }

    @Test
    fun speaksTheDevicesLanguages() {
        assertEquals(35, MathAccessibility.languages.size)
        assertEquals("xの二乗", MathAccessibility.speech("x^2", language = "ja"))
    }
}
