package dev.mathcore

/** Raw JNI surface. Use [MathEngine] instead. */
internal object NativeBridge {
    init {
        System.loadLibrary("mathcore_android")
    }

    external fun create(font: ByteArray): Long
    external fun createBundled(): Long
    external fun destroy(handle: Long)
    external fun unitsPerEm(handle: Long): Float
    external fun render(
        handle: Long,
        tex: String,
        fontSizePx: Float,
        display: Boolean,
        argb: Int,
        macros: String?,
        maxWidthPx: Float,
    ): FloatArray?
    external fun lastError(): String?
    external fun glyphOutline(handle: Long, glyph: Int): FloatArray?
}
