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

/* Formulas are capped by a default budget (256 KB of source after macro
 * expansion, 50k symbols, 200k drawable items) so untrusted input fails
 * cleanly instead of exhausting memory. Rendering past a cap returns NULL and
 * math_last_error() explains which one. Use the Rust API to change them. */

typedef struct MathEngine MathEngine;

enum MathItemKind { MATH_ITEM_GLYPH = 0, MATH_ITEM_RULE = 1, MATH_ITEM_LINE = 2 };

typedef struct MathItem {
    uint8_t kind;      /* MathItemKind */
    uint16_t font;     /* which font of the chain: 0 is the primary (GLYPH only) */
    uint16_t glyph;    /* glyph index in that font (GLYPH only) */
    float x;           /* GLYPH: baseline origin x. RULE: left. LINE: x1 */
    float y;           /* GLYPH: baseline origin y. RULE: top.  LINE: y1 */
    float w;           /* GLYPH: em size in px. RULE: width. LINE: x2 */
    float h;           /* RULE: height. LINE: y2 */
    float thickness;   /* LINE only */
    uint32_t color;    /* 0xRRGGBBAA */
} MathItem;

/* Where a piece of the source ended up on screen, for hit testing and
 * selection. Regions nest; a point usually falls in several, and the smallest
 * is the innermost sub-expression. */
typedef struct MathRegion {
    uint32_t start;   /* byte range of the source that produced this piece */
    uint32_t end;
    float x;          /* bounding box, same pixel space as MathItem */
    float y;
    float width;
    float height;
    uint16_t depth;   /* nesting level, 0 is a top-level atom */
} MathRegion;

typedef struct MathResult {
    float width;
    float ascent;   /* top of bounding box to baseline */
    float descent;  /* baseline to bottom of bounding box */
    size_t count;
    const MathItem* items;
    size_t region_count;          /* zero unless hit_testing was requested */
    const MathRegion* regions;    /* outermost first */
} MathResult;

/* A formula may draw from more than one font: the bundled engine pairs Latin
 * Modern Math with a small slice of STIX Two for the symbols it predates, and a
 * custom font keeps that slice as a fallback. Every glyph item says which font
 * it came from. */

/* Creates an engine from an OpenType math font (the bytes are copied). Returns NULL on failure. */
MathEngine* math_engine_new(const uint8_t* font_data, size_t font_len);
/* Engine with the bundled Latin Modern Math font (NULL if built without it). */
MathEngine* math_engine_new_bundled(void);
void math_engine_free(MathEngine* engine);

/* Font units per em of one font of the chain, needed to scale its outlines:
 * px = units * (item.w / upem). A fallback font may differ from the primary. */
float math_engine_units_per_em(const MathEngine* engine, uint16_t font);

/* Renders `tex`. `macros` is optional: newline-separated "\name=body" definitions.
 * `color` is 0xRRGGBBAA. `max_width` breaks the formula to fit that many pixels;
 * 0 or less renders one line of any width. Returns NULL on error; see
 * math_last_error(). */
MathResult* math_engine_render(const MathEngine* engine, const char* tex, float font_size_px, bool display_mode,
                               uint32_t color, const char* macros, float max_width, bool hit_testing);
void math_result_free(MathResult* result);

/* Glyph outline in font units, y up, as a flat command stream:
 *   0 x y            move
 *   1 x y            line
 *   2 x1 y1 x y      quadratic
 *   3 x1 y1 x2 y2 x y cubic
 *   4                close
 * Returns NULL for glyphs without an outline. Release with math_buffer_free. */
float* math_engine_glyph_outline(const MathEngine* engine, uint16_t font, uint16_t glyph, size_t* out_len);
void math_buffer_free(float* buffer, size_t len);

/* Accessibility. Both parse `tex` and return a string the caller owns and must
 * release with math_string_free; NULL on a parse error. Neither needs an engine
 * or a font. `macros` is optional, as in math_engine_render. */
char* math_mathml(const char* tex, bool display_mode, const char* macros);
char* math_speech(const char* tex, const char* macros);
void math_string_free(char* s);

/* Message for the last failed call on this thread, or NULL. Valid until the next call. */
const char* math_last_error(void);

const char* math_version(void);

#ifdef __cplusplus
}
#endif
#endif
