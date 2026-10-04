// dart:ffi signatures for crates/mathffi/include/mathcore.h.
import 'dart:ffi';

import 'package:ffi/ffi.dart';

final class MathItemStruct extends Struct {
  @Uint8()
  external int kind;
  @Uint16()
  external int font;
  @Uint16()
  external int glyph;
  @Float()
  external double x;
  @Float()
  external double y;
  @Float()
  external double w;
  @Float()
  external double h;
  @Float()
  external double thickness;
  @Uint32()
  external int color;
}

final class MathRegionStruct extends Struct {
  @Uint32()
  external int start;
  @Uint32()
  external int end;
  @Float()
  external double x;
  @Float()
  external double y;
  @Float()
  external double width;
  @Float()
  external double height;
  @Uint16()
  external int depth;
}

final class MathResultStruct extends Struct {
  @Float()
  external double width;
  @Float()
  external double ascent;
  @Float()
  external double descent;
  @Size()
  external int count;
  external Pointer<MathItemStruct> items;
  @Size()
  external int regionCount;
  external Pointer<MathRegionStruct> regions;
}

final class MathEngineOpaque extends Opaque {}

typedef NewC = Pointer<MathEngineOpaque> Function(Pointer<Uint8>, Size);
typedef NewD = Pointer<MathEngineOpaque> Function(Pointer<Uint8>, int);
typedef NewTextC = Pointer<MathEngineOpaque> Function(Pointer<Uint8>, Size, Pointer<Uint8>, Size);
typedef NewTextD = Pointer<MathEngineOpaque> Function(Pointer<Uint8>, int, Pointer<Uint8>, int);
typedef NewBundledC = Pointer<MathEngineOpaque> Function();
typedef FreeC = Void Function(Pointer<MathEngineOpaque>);
typedef FreeD = void Function(Pointer<MathEngineOpaque>);
typedef UpemC = Float Function(Pointer<MathEngineOpaque>, Uint16);
typedef UpemD = double Function(Pointer<MathEngineOpaque>, int);
typedef RenderC = Pointer<MathResultStruct> Function(
    Pointer<MathEngineOpaque>, Pointer<Utf8>, Float, Bool, Uint32, Pointer<Utf8>, Float, Bool);
typedef RenderD = Pointer<MathResultStruct> Function(
    Pointer<MathEngineOpaque>, Pointer<Utf8>, double, bool, int, Pointer<Utf8>, double, bool);
typedef ResultFreeC = Void Function(Pointer<MathResultStruct>);
typedef ResultFreeD = void Function(Pointer<MathResultStruct>);
typedef OutlineC = Pointer<Float> Function(Pointer<MathEngineOpaque>, Uint16, Uint16, Pointer<Size>);
typedef OutlineD = Pointer<Float> Function(Pointer<MathEngineOpaque>, int, int, Pointer<Size>);
typedef BufferFreeC = Void Function(Pointer<Float>, Size);
typedef BufferFreeD = void Function(Pointer<Float>, int);
typedef MathmlC = Pointer<Utf8> Function(Pointer<Utf8>, Bool, Pointer<Utf8>);
typedef MathmlD = Pointer<Utf8> Function(Pointer<Utf8>, bool, Pointer<Utf8>);
typedef SpeechC = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef StringFreeC = Void Function(Pointer<Utf8>);
typedef StringFreeD = void Function(Pointer<Utf8>);
typedef SpeechExC = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Int32);
typedef SpeechExD = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, int);
typedef AsciiC = Pointer<Utf8> Function(Pointer<Utf8>);
typedef SpeechLangC = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, Int32, Pointer<Utf8>);
typedef SpeechLangD = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>, int, Pointer<Utf8>);
typedef BudgetC = Void Function(Pointer<MathEngineOpaque>, Size, Size, Size);
typedef BudgetD = void Function(Pointer<MathEngineOpaque>, int, int, int);
typedef CacheC = Void Function(Pointer<MathEngineOpaque>, Size);
typedef CacheD = void Function(Pointer<MathEngineOpaque>, int);
typedef LastErrorC = Pointer<Utf8> Function();
final class MathEditorOpaque extends Opaque {}

typedef EdNewC = Pointer<MathEditorOpaque> Function();
typedef EdFreeC = Void Function(Pointer<MathEditorOpaque>);
typedef EdFreeD = void Function(Pointer<MathEditorOpaque>);
typedef EdTextC = Void Function(Pointer<MathEditorOpaque>, Pointer<Utf8>);
typedef EdTextD = void Function(Pointer<MathEditorOpaque>, Pointer<Utf8>);
typedef EdGetC = Pointer<Utf8> Function(Pointer<MathEditorOpaque>);
typedef EdKeyC = Bool Function(Pointer<MathEditorOpaque>, Pointer<Utf8>, Bool, Bool);
typedef EdKeyD = bool Function(Pointer<MathEditorOpaque>, Pointer<Utf8>, bool, bool);
typedef EdRenderC = Pointer<MathResultStruct> Function(Pointer<MathEditorOpaque>, Pointer<MathEngineOpaque>, Float, Bool, Uint32);
typedef EdRenderD = Pointer<MathResultStruct> Function(Pointer<MathEditorOpaque>, Pointer<MathEngineOpaque>, double, bool, int);
typedef EdCaretC = Void Function(Pointer<MathEditorOpaque>, Pointer<Float>);
typedef EdCaretD = void Function(Pointer<MathEditorOpaque>, Pointer<Float>);
typedef EdSelC = Pointer<Float> Function(Pointer<MathEditorOpaque>, Pointer<Size>);
typedef EdTapC = Void Function(Pointer<MathEditorOpaque>, Float, Float);
typedef EdTapD = void Function(Pointer<MathEditorOpaque>, double, double);
typedef EdSpeechC = Pointer<Utf8> Function(Pointer<MathEditorOpaque>, Pointer<Utf8>);
typedef AddFontC = Int32 Function(Pointer<MathEngineOpaque>, Pointer<Uint8>, Size, Uint32);
typedef AddFontD = int Function(Pointer<MathEngineOpaque>, Pointer<Uint8>, int, int);
typedef MissingC = Pointer<Utf8> Function(Pointer<MathEngineOpaque>, Pointer<Utf8>, Bool, Pointer<Utf8>);
typedef MissingD = Pointer<Utf8> Function(Pointer<MathEngineOpaque>, Pointer<Utf8>, bool, Pointer<Utf8>);
typedef SystemFontsC = Int32 Function(Pointer<MathEngineOpaque>, Pointer<Utf8>, Bool, Pointer<Utf8>);
typedef SystemFontsD = int Function(Pointer<MathEngineOpaque>, Pointer<Utf8>, bool, Pointer<Utf8>);
typedef VersionC = Pointer<Utf8> Function();

class MathBindings {
  MathBindings(DynamicLibrary lib)
      : engineNew = lib.lookupFunction<NewC, NewD>('math_engine_new'),
        engineNewBundled = lib.lookupFunction<NewBundledC, NewBundledC>('math_engine_new_bundled'),
        engineNewWithText = lib.lookupFunction<NewTextC, NewTextD>('math_engine_new_with_text_font'),
        engineFree = lib.lookupFunction<FreeC, FreeD>('math_engine_free'),
        unitsPerEm = lib.lookupFunction<UpemC, UpemD>('math_engine_units_per_em'),
        render = lib.lookupFunction<RenderC, RenderD>('math_engine_render'),
        resultFree = lib.lookupFunction<ResultFreeC, ResultFreeD>('math_result_free'),
        glyphOutline = lib.lookupFunction<OutlineC, OutlineD>('math_engine_glyph_outline'),
        bufferFree = lib.lookupFunction<BufferFreeC, BufferFreeD>('math_buffer_free'),
        mathml = lib.lookupFunction<MathmlC, MathmlD>('math_mathml'),
        speech = lib.lookupFunction<SpeechC, SpeechC>('math_speech'),
        speechEx = lib.lookupFunction<SpeechExC, SpeechExD>('math_speech_ex'),
        speechTree = lib.lookupFunction<SpeechExC, SpeechExD>('math_speech_tree'),
        asciimathToTex = lib.lookupFunction<AsciiC, AsciiC>('math_asciimath_to_tex'),
        nemeth = lib.lookupFunction<SpeechC, SpeechC>('math_nemeth'),
        speechLang = lib.lookupFunction<SpeechLangC, SpeechLangD>('math_speech_lang'),
        speechTreeLang = lib.lookupFunction<SpeechLangC, SpeechLangD>('math_speech_tree_lang'),
        setBudget = lib.lookupFunction<BudgetC, BudgetD>('math_engine_set_budget'),
        setCacheCapacity = lib.lookupFunction<CacheC, CacheD>('math_engine_set_cache_capacity'),
        stringFree = lib.lookupFunction<StringFreeC, StringFreeD>('math_string_free'),
        lastError = lib.lookupFunction<LastErrorC, LastErrorC>('math_last_error'),
        version = lib.lookupFunction<VersionC, VersionC>('math_version'),
        addFont = lib.lookupFunction<AddFontC, AddFontD>('math_engine_add_font'),
        missingChars = lib.lookupFunction<MissingC, MissingD>('math_engine_missing_chars'),
        useSystemFonts = lib.lookupFunction<SystemFontsC, SystemFontsD>('math_engine_use_system_fonts'),
        speechLanguages = lib.lookupFunction<VersionC, VersionC>('math_speech_languages'),
        editorNew = lib.lookupFunction<EdNewC, EdNewC>('math_editor_new'),
        editorFree = lib.lookupFunction<EdFreeC, EdFreeD>('math_editor_free'),
        editorSetTex = lib.lookupFunction<EdTextC, EdTextD>('math_editor_set_tex'),
        editorTex = lib.lookupFunction<EdGetC, EdGetC>('math_editor_tex'),
        editorSelectedTex = lib.lookupFunction<EdGetC, EdGetC>('math_editor_selected_tex'),
        editorType = lib.lookupFunction<EdTextC, EdTextD>('math_editor_type'),
        editorInsertTex = lib.lookupFunction<EdTextC, EdTextD>('math_editor_insert_tex'),
        editorCommand = lib.lookupFunction<EdTextC, EdTextD>('math_editor_command'),
        editorKey = lib.lookupFunction<EdKeyC, EdKeyD>('math_editor_key'),
        editorRender = lib.lookupFunction<EdRenderC, EdRenderD>('math_editor_render'),
        editorCaret = lib.lookupFunction<EdCaretC, EdCaretD>('math_editor_caret'),
        editorSelection = lib.lookupFunction<EdSelC, EdSelC>('math_editor_selection'),
        editorTap = lib.lookupFunction<EdTapC, EdTapD>('math_editor_tap'),
        editorDescribe = lib.lookupFunction<EdSpeechC, EdSpeechC>('math_editor_describe'),
        editorSpeech = lib.lookupFunction<EdSpeechC, EdSpeechC>('math_editor_speech');

  final NewD engineNew;
  final NewBundledC engineNewBundled;
  final NewTextD engineNewWithText;
  final FreeD engineFree;
  final UpemD unitsPerEm;
  final RenderD render;
  final ResultFreeD resultFree;
  final OutlineD glyphOutline;
  final BufferFreeD bufferFree;
  final MathmlD mathml;
  final SpeechC speech;
  final SpeechExD speechEx;
  final SpeechExD speechTree;
  final AsciiC asciimathToTex;
  final SpeechC nemeth;
  final SpeechLangD speechLang;
  final SpeechLangD speechTreeLang;
  final BudgetD setBudget;
  final CacheD setCacheCapacity;
  final AddFontD addFont;
  final MissingD missingChars;
  final SystemFontsD useSystemFonts;
  final VersionC speechLanguages;
  final EdNewC editorNew;
  final EdFreeD editorFree;
  final EdTextD editorSetTex;
  final EdGetC editorTex;
  final EdGetC editorSelectedTex;
  final EdTextD editorType;
  final EdTextD editorInsertTex;
  final EdTextD editorCommand;
  final EdKeyD editorKey;
  final EdRenderD editorRender;
  final EdCaretD editorCaret;
  final EdSelC editorSelection;
  final EdTapD editorTap;
  final EdSpeechC editorDescribe;
  final EdSpeechC editorSpeech;
  final StringFreeD stringFree;
  final LastErrorC lastError;
  final VersionC version;
}
