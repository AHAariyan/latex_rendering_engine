package com.example.mathcoredemo

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import dev.mathcore.MathRegion
import dev.mathcore.MathText

private val samples = listOf(
    "Quadratic formula" to "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}",
    "Basel problem" to "\\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6}",
    "Gaussian integral" to "\\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx = \\sqrt{\\pi}",
    "Euler" to "e^{i\\pi} + 1 = 0",
    "Cases" to "f(x) = \\begin{cases} x^2 & \\text{if } x \\ge 0 \\\\ -x & \\text{otherwise} \\end{cases}",
    "Matrix" to "A = \\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix} \\in \\mathbb{R}^{2\\times 2}",
    "Schrödinger" to "i\\hbar \\frac{\\partial}{\\partial t} \\Psi = \\left[ -\\frac{\\hbar^2}{2m} \\nabla^2 + V \\right] \\Psi",
    "Limits" to "\\lim_{x \\to 0} \\frac{\\sin x}{x} = 1, \\quad \\binom{n}{k} = \\frac{n!}{k!(n-k)!}",
    "Colors and boxes" to "\\boxed{E = mc^2} \\quad \\textcolor{red}{\\alpha} + \\textcolor{blue}{\\beta}",
    "Braces and arrows" to "\\underbrace{a + b + c}_{3} \\xrightarrow{\\ f\\ } \\overbrace{d}^{1}",
    "Wrapped to the screen" to "f(x) = a_0 + a_1 x + a_2 x^2 + a_3 x^3 + a_4 x^4 + a_5 x^5 + a_6 x^6 + a_7 x^7",
    "Wrapped chain" to "\\int_0^1 f(x)\\,dx = \\lim_{n\\to\\infty} \\sum_{i=1}^{n} f(x_i)\\Delta x = F(1) - F(0)",
)

@Composable
fun DemoScreen() {
    var input by remember { mutableStateOf("\\frac{a}{b} + \\sqrt{x^2 + y^2}") }
    var error by remember { mutableStateOf<String?>(null) }
    var tapped by remember { mutableStateOf<String?>(null) }
    LazyColumn(modifier = Modifier.safeDrawingPadding().padding(horizontal = 16.dp)) {
        item {
            Text("mathcore demo", style = MaterialTheme.typography.headlineSmall, modifier = Modifier.padding(vertical = 12.dp))
            OutlinedTextField(
                value = input,
                onValueChange = { input = it; error = null },
                label = { Text("Type TeX") },
                modifier = Modifier.fillMaxWidth(),
            )
            Spacer(Modifier.height(12.dp))
            MathText(
                latex = input,
                fontSize = 26.sp,
                color = MaterialTheme.colorScheme.onBackground,
                modifier = Modifier.fillMaxWidth(),
                onError = { error = it },
                onTap = { r: MathRegion -> tapped = r.textIn(input) },
            )
            error?.let { Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodySmall) }
            Text(
                tapped?.let { "tapped: $it" } ?: "tap the formula to inspect a sub-expression",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.secondary,
            )
            Spacer(Modifier.height(12.dp))
            HorizontalDivider()
        }
        items(samples) { (title, tex) ->
            Column(modifier = Modifier.padding(vertical = 10.dp)) {
                Text(title, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.secondary)
                Spacer(Modifier.height(6.dp))
                MathText(
                    latex = tex,
                    fontSize = 22.sp,
                    color = MaterialTheme.colorScheme.onBackground,
                    modifier = Modifier.fillMaxWidth(),
                )
            }
        }
    }
}
