package dev.mathcore

import android.os.Build
import java.io.File

/**
 * Finds the device font for a script the engine's fonts lack. Android ships a
 * Noto face for every script it can display (NotoSansBengali, NotoNaskhArabic,
 * NotoSansCJK...), so the font is chosen by the script of the character.
 */
internal object SystemFontFinder {
    class Found(val file: File, val index: Int) {
        val key: String get() = "${file.path}#$index"
    }

    /** File-name keywords of the fonts for a code point's script, best first. */
    private fun keywords(cp: Int): List<String> = when (cp) {
        in 0x0370..0x03FF, in 0x1F00..0x1FFF -> listOf("Roboto-Regular", "NotoSerif-Regular")
        in 0x0400..0x052F, in 0x1E00..0x1EFF, in 0x00C0..0x024F -> listOf("Roboto-Regular", "NotoSerif-Regular")
        in 0x0530..0x058F -> listOf("Armenian")
        in 0x0590..0x05FF -> listOf("Hebrew")
        in 0x0600..0x06FF, in 0x0750..0x077F, in 0x08A0..0x08FF, in 0xFB50..0xFDFF, in 0xFE70..0xFEFF -> listOf("NaskhArabic", "Arabic")
        in 0x0700..0x074F -> listOf("Syriac")
        in 0x0780..0x07BF -> listOf("Thaana")
        in 0x0900..0x097F -> listOf("Devanagari")
        in 0x0980..0x09FF -> listOf("Bengali")
        in 0x0A00..0x0A7F -> listOf("Gurmukhi")
        in 0x0A80..0x0AFF -> listOf("Gujarati")
        in 0x0B00..0x0B7F -> listOf("Oriya")
        in 0x0B80..0x0BFF -> listOf("Tamil")
        in 0x0C00..0x0C7F -> listOf("Telugu")
        in 0x0C80..0x0CFF -> listOf("Kannada")
        in 0x0D00..0x0D7F -> listOf("Malayalam")
        in 0x0D80..0x0DFF -> listOf("Sinhala")
        in 0x0E00..0x0E7F -> listOf("Thai")
        in 0x0E80..0x0EFF -> listOf("Lao")
        in 0x0F00..0x0FFF -> listOf("Tibetan")
        in 0x1000..0x109F -> listOf("Myanmar")
        in 0x10A0..0x10FF -> listOf("Georgian")
        in 0x1200..0x139F -> listOf("Ethiopic")
        in 0x1780..0x17FF -> listOf("Khmer")
        in 0x1100..0x11FF, in 0x3130..0x318F, in 0xAC00..0xD7AF -> listOf("CJK", "Korean")
        in 0x2E80..0x9FFF, in 0xF900..0xFAFF, in 0xFF00..0xFFEF, in 0x20000..0x2FFFF -> listOf("CJK", "Han")
        else -> listOf("NotoSans-Regular", "Roboto-Regular", "NotoSansSymbols")
    }

    /** Every font file on the device with its face index in a collection. */
    private val fonts: List<Found> by lazy {
        if (Build.VERSION.SDK_INT >= 29) {
            android.graphics.fonts.SystemFonts.getAvailableFonts().mapNotNull { f -> f.file?.let { Found(it, f.ttcIndex) } }
        } else {
            File("/system/fonts").listFiles().orEmpty().filter { it.name.endsWith(".ttf") || it.name.endsWith(".otf") || it.name.endsWith(".ttc") }.map { Found(it, 0) }
        }.distinctBy { it.key }
    }

    fun find(codePoint: Int, tried: Set<String>): Found? {
        for (k in keywords(codePoint)) {
            val match = fonts
                .filter { it.key !in tried && it.file.name.contains(k, ignoreCase = true) }
                // Regular weight, the text (not UI) cut, Sans before Serif.
                .sortedWith(compareBy({ !it.file.name.contains("Regular") }, { it.file.name.contains("UI") }, { it.file.name.contains("Serif") }, { it.index }))
                .firstOrNull()
            if (match != null) return match
        }
        return null
    }
}
