package dev.mathcore

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.size
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.drawIntoCanvas
import androidx.compose.ui.graphics.nativeCanvas
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp

/**
 * Typesets [latex] natively.
 *
 * With [wrap] on, a formula too wide for the space the parent offers is broken
 * into lines before relations and binary operators. With it off the formula
 * keeps its natural width, which suits a horizontally scrollable row.
 *
 * TalkBack reads the whole formula, then lets the user step through its parts
 * (terms, fractions, scripts), each outlined where it is drawn.
 *
 * [onTap] receives the smallest sub-expression under the finger, whose `start`
 * and `end` are UTF-8 byte offsets into [latex] (see [MathRegion.textIn]).
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
    speechVerbosity: SpeechVerbosity = SpeechVerbosity.Brief,
    onError: ((String) -> Unit)? = null,
    onTap: ((MathRegion) -> Unit)? = null,
) {
    BoxWithConstraints(modifier) {
        val density = LocalDensity.current
        val sizePx = with(density) { fontSize.toPx() }
        val maxWidthPx = if (wrap && constraints.hasBoundedWidth) constraints.maxWidth.toFloat() else 0f
        val argb = color.toArgb()
        val result = remember(latex, sizePx, argb, displayMode, macros, engine, maxWidthPx) {
            runCatching {
                if (latex.isBlank()) null else engine.render(latex, sizePx, displayMode, argb, macros, maxWidthPx, hitTesting = true)
            }
        }
        val error = (result.exceptionOrNull() as? MathParseException)?.message
        LaunchedEffect(error) { if (error != null) onError?.invoke(error) }
        val layout = result.getOrNull()
        val spoken = remember(latex, speechVerbosity) { MathAccessibility.speechOrNull(latex, speechVerbosity) }
        val parts = remember(latex, speechVerbosity, layout) {
            if (layout == null) emptyList()
            else MathAccessibility.speechTreeOrNull(latex, speechVerbosity)?.children?.takeIf { it.size > 1 }.orEmpty()
        }
        val w = with(density) { (layout?.width ?: 0f).toDp() }
        val h = with(density) { (layout?.height ?: 0f).toDp() }
        Box(Modifier.size(w, h)) {
            Canvas(
                modifier = Modifier
                    .size(w, h)
                    .semantics { spoken?.let { contentDescription = it } }
                    .then(
                        if (onTap == null || layout == null) {
                            Modifier
                        } else {
                            Modifier.pointerInput(layout) {
                                detectTapGestures { p -> layout.hitNearest(p.x, p.y)?.let(onTap) }
                            }
                        },
                    ),
            ) {
                drawIntoCanvas { engine.draw(layout ?: return@drawIntoCanvas, it.nativeCanvas) }
            }
            // One invisible node per part, placed over it, for TalkBack to step through.
            if (layout != null) {
                for (part in parts) {
                    val regions = layout.highlight(part.start, part.end)
                    if (regions.isEmpty()) continue
                    val left = regions.minOf { it.x }
                    val top = regions.minOf { it.y }
                    val pw = with(density) { (regions.maxOf { it.x + it.width } - left).toDp() }
                    val ph = with(density) { (regions.maxOf { it.y + it.height } - top).toDp() }
                    Box(
                        Modifier
                            .offset { IntOffset(left.toInt(), top.toInt()) }
                            .size(pw, ph)
                            .semantics { contentDescription = part.announcement },
                    )
                }
            }
        }
    }
}
