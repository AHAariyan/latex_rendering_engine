// dart:ffi signatures for crates/mathffi/include/mathcore.h.
import 'dart:ffi';

import 'package:ffi/ffi.dart';

final class MathItemStruct extends Struct {
  @Uint8()
  external int kind;
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
}

final class MathEngineOpaque extends Opaque {}

typedef NewC = Pointer<MathEngineOpaque> Function(Pointer<Uint8>, Size);
typedef NewD = Pointer<MathEngineOpaque> Function(Pointer<Uint8>, int);
typedef NewBundledC = Pointer<MathEngineOpaque> Function();
typedef FreeC = Void Function(Pointer<MathEngineOpaque>);
typedef FreeD = void Function(Pointer<MathEngineOpaque>);
typedef UpemC = Float Function(Pointer<MathEngineOpaque>);
typedef UpemD = double Function(Pointer<MathEngineOpaque>);
typedef RenderC = Pointer<MathResultStruct> Function(
    Pointer<MathEngineOpaque>, Pointer<Utf8>, Float, Bool, Uint32, Pointer<Utf8>, Float);
typedef RenderD = Pointer<MathResultStruct> Function(
    Pointer<MathEngineOpaque>, Pointer<Utf8>, double, bool, int, Pointer<Utf8>, double);
typedef ResultFreeC = Void Function(Pointer<MathResultStruct>);
typedef ResultFreeD = void Function(Pointer<MathResultStruct>);
typedef OutlineC = Pointer<Float> Function(Pointer<MathEngineOpaque>, Uint16, Pointer<Size>);
typedef OutlineD = Pointer<Float> Function(Pointer<MathEngineOpaque>, int, Pointer<Size>);
typedef BufferFreeC = Void Function(Pointer<Float>, Size);
typedef BufferFreeD = void Function(Pointer<Float>, int);
typedef MathmlC = Pointer<Utf8> Function(Pointer<Utf8>, Bool, Pointer<Utf8>);
typedef MathmlD = Pointer<Utf8> Function(Pointer<Utf8>, bool, Pointer<Utf8>);
typedef SpeechC = Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>);
typedef StringFreeC = Void Function(Pointer<Utf8>);
typedef StringFreeD = void Function(Pointer<Utf8>);
typedef LastErrorC = Pointer<Utf8> Function();
typedef VersionC = Pointer<Utf8> Function();

class MathBindings {
  MathBindings(DynamicLibrary lib)
      : engineNew = lib.lookupFunction<NewC, NewD>('math_engine_new'),
        engineNewBundled = lib.lookupFunction<NewBundledC, NewBundledC>('math_engine_new_bundled'),
        engineFree = lib.lookupFunction<FreeC, FreeD>('math_engine_free'),
        unitsPerEm = lib.lookupFunction<UpemC, UpemD>('math_engine_units_per_em'),
        render = lib.lookupFunction<RenderC, RenderD>('math_engine_render'),
        resultFree = lib.lookupFunction<ResultFreeC, ResultFreeD>('math_result_free'),
        glyphOutline = lib.lookupFunction<OutlineC, OutlineD>('math_engine_glyph_outline'),
        bufferFree = lib.lookupFunction<BufferFreeC, BufferFreeD>('math_buffer_free'),
        mathml = lib.lookupFunction<MathmlC, MathmlD>('math_mathml'),
        speech = lib.lookupFunction<SpeechC, SpeechC>('math_speech'),
        stringFree = lib.lookupFunction<StringFreeC, StringFreeD>('math_string_free'),
        lastError = lib.lookupFunction<LastErrorC, LastErrorC>('math_last_error'),
        version = lib.lookupFunction<VersionC, VersionC>('math_version');

  final NewD engineNew;
  final NewBundledC engineNewBundled;
  final FreeD engineFree;
  final UpemD unitsPerEm;
  final RenderD render;
  final ResultFreeD resultFree;
  final OutlineD glyphOutline;
  final BufferFreeD bufferFree;
  final MathmlD mathml;
  final SpeechC speech;
  final StringFreeD stringFree;
  final LastErrorC lastError;
  final VersionC version;
}
