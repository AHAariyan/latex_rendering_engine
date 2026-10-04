package dev.mathcore

import android.graphics.Bitmap
import android.graphics.Canvas
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/** MathField on a device: the soft keyboard connection, hardware keys, drawing. */
@RunWith(AndroidJUnit4::class)
class MathFieldTest {
    private fun field(): MathField {
        lateinit var f: MathField
        InstrumentationRegistry.getInstrumentation().runOnMainSync {
            f = MathField(InstrumentationRegistry.getInstrumentation().targetContext)
        }
        return f
    }

    private fun onMain(block: () -> Unit) = InstrumentationRegistry.getInstrumentation().runOnMainSync(block)

    @Test
    fun softKeyboardInputBuildsAFormula() {
        val f = field()
        val changes = ArrayList<String>()
        onMain {
            f.onChange = { changes.add(it) }
            val ic = f.onCreateInputConnection(EditorInfo())
            for (ch in "1/2") ic.commitText(ch.toString(), 1)
            assertEquals("\\frac{1}{2}", f.latex)
            ic.deleteSurroundingText(1, 0)
            assertEquals("\\frac{1}{}", f.latex)
            ic.commitText("3", 1)
            ic.commitText("\n", 1)
            ic.commitText("+x", 1)
        }
        assertEquals("\\frac{1}{3}+x", f.latex)
        assertEquals("\\frac{1}{3}+x", changes.last())
        assertEquals(MathEditor(f.latex).use { it.speech() }, f.contentDescription.toString())
    }

    @Test
    fun hardwareKeysMoveSelectAndUndo() {
        val f = field()
        fun key(code: Int, meta: Int = 0) = f.onKeyDown(code, KeyEvent(0, 0, KeyEvent.ACTION_DOWN, code, 0, meta))
        onMain {
            f.latex = "ab"
            key(KeyEvent.KEYCODE_DPAD_LEFT)
            key(KeyEvent.KEYCODE_X)
            assertEquals("axb", f.latex)
            key(KeyEvent.KEYCODE_DEL)
            assertEquals("ab", f.latex)
            key(KeyEvent.KEYCODE_A, KeyEvent.META_CTRL_ON)
            key(KeyEvent.KEYCODE_DEL)
            assertEquals("", f.latex)
            key(KeyEvent.KEYCODE_Z, KeyEvent.META_CTRL_ON)
            assertEquals("ab", f.latex)
            f.isEditable = false
            key(KeyEvent.KEYCODE_Y)
            assertEquals("ab", f.latex)
        }
    }

    @Test
    fun measuresToTheFormulaAndDraws() {
        val f = field()
        onMain {
            f.textSizePx = 60f
            val spec = View.MeasureSpec.makeMeasureSpec(0, View.MeasureSpec.UNSPECIFIED)
            f.measure(spec, spec)
            val emptyWidth = f.measuredWidth
            f.latex = "\\frac{a+b}{c} + \\sqrt{x}"
            f.measure(spec, spec)
            assertTrue(f.measuredWidth > emptyWidth)
            f.layout(0, 0, f.measuredWidth, f.measuredHeight)
            val bmp = Bitmap.createBitmap(f.measuredWidth, f.measuredHeight, Bitmap.Config.ARGB_8888)
            f.draw(Canvas(bmp))
            val pixels = IntArray(bmp.width * bmp.height)
            bmp.getPixels(pixels, 0, bmp.width, 0, 0, bmp.width, bmp.height)
            assertTrue("something was drawn", pixels.any { it != 0 })
            // A tap at the left edge moves the cursor to the start.
            f.editor.tap(0f, f.measuredHeight / 2f)
            f.type("y")
            assertTrue(f.latex.startsWith("y"))
        }
    }
}
