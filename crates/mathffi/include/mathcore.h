/* mathcore C ABI. See crates/mathffi/src/lib.rs for the contract.
 *
 * Lifetimes: every pointer returned by a *_new / *_render / *_outline call is
 * owned by the caller and must be released with the matching *_free call.
 * Strings are UTF-8 and NUL-terminated. All functions are thread-safe as long
 * as one MathEngine is not used from two threads at the same time.
 */
#ifndef MATHCORE_H
#define MATHCORE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MathEngine MathEngine;

enum MathItemKind { MATH_ITEM_GLYPH = 0, MATH_ITEM_RULE = 1, MATH_ITEM_LINE = 2 };

typedef struct MathItem {
    uint8_t kind;      /* MathItemKind */
    uint16_t glyph;    /* glyph index in the engine's font (GLYPH only) */
    float x;           /* GLYPH: baseline origin x. RULE: left. LINE: x1 */
    float y;           /* GLYPH: baseline origin y. RULE: top.  LINE: y1 */
    float w;           /* GLYPH: em size in px. RULE: width. LINE: x2 */
    float h;           /* RULE: height. LINE: y2 */
    float thickness;   /* LINE only */
    uint32_t color;    /* 0xRRGGBBAA */
} MathItem;

typedef struct MathResult {
    float width;
    float ascent;   /* top of bounding box to baseline */
    float descent;  /* baseline to bottom of bounding box */
    size_t count;
    const MathItem* items;
} MathResult;

/* Creates an engine from an OpenType math font (the bytes are copied). Returns NULL on failure. */
MathEngine* math_engine_new(const uint8_t* font_data, size_t font_len);
void math_engine_free(MathEngine* engine);

/* Font units per em, needed to scale glyph outlines: px = units * (item.w / upem). */
float math_engine_units_per_em(const MathEngine* engine);

/* Renders `tex`. `macros` is optional: newline-separated "\name=body" definitions.
 * `color` is 0xRRGGBBAA. Returns NULL on error; see math_last_error(). */
MathResult* math_engine_render(const MathEngine* engine, const char* tex, float font_size_px, bool display_mode,
                               uint32_t color, const char* macros);
void math_result_free(MathResult* result);

/* Glyph outline in font units, y up, as a flat command stream:
 *   0 x y            move
 *   1 x y            line
 *   2 x1 y1 x y      quadratic
 *   3 x1 y1 x2 y2 x y cubic
 *   4                close
 * Returns NULL for glyphs without an outline. Release with math_buffer_free. */
float* math_engine_glyph_outline(const MathEngine* engine, uint16_t glyph, size_t* out_len);
void math_buffer_free(float* buffer, size_t len);

/* Message for the last failed call on this thread, or NULL. Valid until the next call. */
const char* math_last_error(void);

const char* math_version(void);

#ifdef __cplusplus
}
#endif
#endif
