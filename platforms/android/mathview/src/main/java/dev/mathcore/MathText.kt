package dev.mathcore

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp

/**
 * Typesets [latex] natively.
 *
 * With [wrap] on, a formula too wide for the space the parent offers is broken
 * into lines before relations and binary operators, the way an author breaks a
 * long equation by hand. With it off the formula keeps its natural width, which
 * suits a horizontally scrollable row.
 *
 * TalkBack reads the formula aloud: the composable carries a spoken rendering
 * as its content description, so `x^2` is announced as "x squared".
 *
 * When the source fails to parse, [onError] receives the message and nothing is drawn.
 */
@Composable
fun MathText(
    latex: String,
    modifier: Modifier = Modifier,
    fontSize: TextUnit = 18.sp,
    color: Color = Color.Black,
    displayMode: Boolean = true,
    wrap: Boolean = true,
    macros: Map<String, String> = emptyMap(),
    engine: MathEngine = MathEngine.shared,
    onError: ((String) -> Unit)? = null,
) {
    BoxWithConstraints(modifier) {
        val density = LocalDensity.current
        val sizePx = with(density) { fontSize.toPx() }
        val maxWidthPx = if (wrap && constraints.hasBoundedWidth) constraints.maxWidth.toFloat() else 0f
        val argb = color.toArgb()
        val layout = remember(latex, sizePx, argb, displayMode, macros, engine, maxWidthPx) {
            try {
                if (latex.isBlank()) null else engine.render(latex, sizePx, displayMode, argb, macros, maxWidthPx)
            } catch (e: MathParseException) {
                onError?.invoke(e.message ?: "parse error")
                null
            }
        }
        val w = with(density) { (layout?.width ?: 0f).toDp() }
        val h = with(density) { (layout?.height ?: 0f).toDp() }
        val spoken = remember(latex) { MathAccessibility.speechOrNull(latex) }
        Canvas(
            modifier = Modifier.size(w, h).semantics { spoken?.let { contentDescription = it } },
        ) {
            val l = layout ?: return@Canvas
            drawIntoCanvas { engine.draw(l, it.nativeCanvas) }
        }
    }
}
