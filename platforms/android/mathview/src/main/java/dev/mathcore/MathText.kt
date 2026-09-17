package dev.mathcore

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp

/**
 * Typesets [latex] natively. Sized to the formula; use [modifier] for padding.
 * When the source fails to parse, [onError] receives the message and nothing is drawn.
 */
@Composable
fun MathText(
    latex: String,
    modifier: Modifier = Modifier,
    fontSize: TextUnit = 18.sp,
    color: Color = Color.Black,
    displayMode: Boolean = true,
    macros: Map<String, String> = emptyMap(),
    engine: MathEngine = MathEngine.shared,
    onError: ((String) -> Unit)? = null,
) {
    val density = LocalDensity.current
    val sizePx = with(density) { fontSize.toPx() }
    val argb = color.toArgb()
    val layout = remember(latex, sizePx, argb, displayMode, macros, engine) {
        try {
            if (latex.isBlank()) null else engine.render(latex, sizePx, displayMode, argb, macros)
        } catch (e: MathParseException) {
            onError?.invoke(e.message ?: "parse error")
            null
        }
    }
    val (w, h) = with(density) { (layout?.width ?: 0f).toDp() to (layout?.height ?: 0f).toDp() }
    Canvas(modifier = modifier.size(w, h)) {
        val l = layout ?: return@Canvas
        drawIntoCanvas { engine.draw(l, it.nativeCanvas) }
    }
}
