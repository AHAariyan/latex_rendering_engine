package dev.mathcore

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Runs the Kotlin bindings against the real engine on the host JVM: the
 * pipeline builds libmathcore_android for the host and points
 * java.library.path at it. Android's graphics classes are inert stubs here,
 * so these tests cover everything except drawing.
 */
class BindingTest {
    private val engine = MathEngine.shared

    @Test fun rendersAndReportsSize() {
        val layout = engine.render("x = \\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}", 32f)
        assertTrue(layout.width > 100f)
        assertTrue(layout.itemCount > 10)
    }

    @Test fun parseErrorsThrow() {
        assertThrows(MathParseException::class.java) { engine.render("\\nosuchcommand", 20f) }
    }

    @Test fun hitTestingMapsBackToNonAsciiSource() {
        val tex = "α ≤ \\frac{a}{b}"
        val layout = engine.render(tex, 32f, hitTesting = true)
        val frac = layout.regions.maxByOrNull { it.width * it.height }!!
        assertEquals("\\frac{a}{b}", frac.textIn(tex))
        assertEquals(1, layout.highlight(frac.start, frac.end).size)
    }

    @Test fun speechTreeAndVerbosity() {
        assertEquals("the fraction 1 over 2, end fraction", MathAccessibility.speech("\\frac{1}{2}", SpeechVerbosity.Verbose, "en"))
        val tree = MathAccessibility.speechTree("\\frac{a+b}{c} = 1", language = "en")
        assertEquals("formula", tree.role)
        assertEquals("numerator", tree.children[0].children[0].label)
        assertEquals("numerator: a plus b", tree.children[0].children[0].announcement)
    }

    @Test fun speechInOtherLanguages() {
        assertEquals("x al cuadrado", MathAccessibility.speech("x^2", language = "es"))
        assertEquals("x এর বর্গ", MathAccessibility.speech("x^2", language = "bn-BD"))
        assertEquals("xの二乗", MathAccessibility.speech("x^2", language = "ja"))
        assertEquals("x squared", MathAccessibility.speech("x^2", language = "xx"))
        assertEquals("⠭⠘⠆", MathAccessibility.nemeth("x^2"))
    }

    @Test fun asciimathChemistryText() {
        assertEquals("\\frac{x}{y}", MathEngine.asciimathToTex("x/y"))
        assertNotNull(engine.render("\\ce{2H2 + O2 -> 2H2O}", 20f))
        assertNotNull(engine.render("\\text{if \$x>0\$ then}", 20f))
    }

    @Test fun budgetAndCache() {
        val e = MathEngine.bundled()
        e.setBudget(maxNodes = 10)
        assertThrows(MathParseException::class.java) { e.render("x+".repeat(50) + "x", 20f) }
        e.setBudget()
        e.setCacheCapacity(0)
        assertNotNull(e.render("x+".repeat(50) + "x", 20f))
        e.close()
    }

    @Test
    fun editorTypesNavigatesAndDescribes() {
        MathEditor().use { e ->
            e.type("x^2")
            e.key(MathEditor.Key.Right)
            e.type("+1/2")
            assertEquals("x^{2}+\\frac{1}{2}", e.latex)
            assertEquals("denominator, 2", e.cursorDescription("en"))
            assertEquals("denominador, 2", e.cursorDescription("es"))
            e.selectAll()
            assertEquals(e.latex, e.selectedLatex)
            e.key(MathEditor.Key.Backspace)
            assertEquals("", e.latex)
            e.undo()
            e.insertLatex("\\sqrt{y}")
            assertEquals("x^{2}+\\frac{1}{2}\\sqrt{y}", e.latex)
        }
    }
}
