package dev.mathcore

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.TextUnit
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView

/**
 * An editable formula for Compose: a text field for mathematics.
 *
 *     var answer by remember { mutableStateOf("") }
 *     MathInput(answer, onValueChange = { answer = it })
 *
 * [value] is TeX. Typing follows the usual conventions (`/` for a fraction,
 * `^` for a superscript, `sqrt`, `pi`...); TalkBack hears the formula and,
 * after each key, where the cursor is.
 */
@Composable
fun MathInput(
    value: String,
    onValueChange: (String) -> Unit,
    modifier: Modifier = Modifier,
    fontSize: TextUnit = 20.sp,
    color: Color = Color.Black,
    accentColor: Color = Color(0xFF1A73E8),
    placeholder: String = "",
    enabled: Boolean = true,
) {
    val px = with(LocalDensity.current) { fontSize.toPx() }
    AndroidView(
        modifier = modifier,
        factory = { context -> MathField(context) },
        update = { field ->
            field.onChange = onValueChange
            field.textSizePx = px
            field.textColor = color.toArgb()
            field.accentColor = accentColor.toArgb()
            field.placeholder = placeholder
            field.isEditable = enabled
            // Only when the caller's value is not what the field just reported.
            if (field.latex != value && MathEditor(value).use { it.latex } != field.latex) field.latex = value
        },
    )
}
