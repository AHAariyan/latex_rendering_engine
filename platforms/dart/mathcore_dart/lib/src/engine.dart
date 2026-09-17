import 'dart:ffi';
import 'dart:io';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

import 'bindings.dart';
import 'layout.dart';

class MathParseException implements Exception {
  MathParseException(this.message);
  final String message;
  @override
  String toString() => 'MathParseException: $message';
}

/// A TeX math typesetting engine backed by the native mathcore library.
///
/// One engine per font. Not thread-safe across isolates; create one per isolate.
class MathEngine {
  MathEngine._(this._b, this._handle);

  /// Override the library location, e.g. for tests: `MathEngine.libraryPath = 'target/release/libmathcore_ffi.so'`.
  static String? libraryPath;

  static MathBindings? _bindings;

  static MathBindings get bindings => _bindings ??= MathBindings(_open());

  static DynamicLibrary _open() {
    final path = libraryPath;
    if (path != null) return DynamicLibrary.open(path);
    if (Platform.isIOS || Platform.isMacOS) return DynamicLibrary.process();
    if (Platform.isWindows) return DynamicLibrary.open('mathcore_ffi.dll');
    return DynamicLibrary.open('libmathcore_ffi.so');
  }

  static String _lastError(MathBindings b) {
    final p = b.lastError();
    return p == nullptr ? 'unknown native error' : p.toDartString();
  }

  /// Engine with the bundled Latin Modern Math font.
  factory MathEngine.bundled() {
    final b = bindings;
    final h = b.engineNewBundled();
    if (h == nullptr) throw StateError(_lastError(b));
    return MathEngine._(b, h);
  }

  /// Engine for any OpenType font with a MATH table.
  factory MathEngine.fromFont(Uint8List font) {
    final b = bindings;
    final buf = malloc<Uint8>(font.length);
    try {
      buf.asTypedList(font.length).setAll(0, font);
      final h = b.engineNew(buf, font.length);
      if (h == nullptr) throw StateError(_lastError(b));
      return MathEngine._(b, h);
    } finally {
      malloc.free(buf);
    }
  }

  final MathBindings _b;
  Pointer<MathEngineOpaque> _handle;
  /// Keyed by font and glyph, since a formula may draw from more than one font.
  final Map<int, GlyphOutline?> _outlines = {};
  final Map<int, double> _upem = {};

  /// Font units per em of one font of the chain; a fallback may differ.
  double unitsPerEm([int font = 0]) => _upem.putIfAbsent(font, () => _b.unitsPerEm(_handle, font));

  static String get nativeVersion => bindings.version().toDartString();

  /// Presentation MathML for [tex], for a screen reader. Needs no engine.
  static String mathml(String tex, {bool displayMode = true}) =>
      _string(tex, (b, t) => b.mathml(t, displayMode, nullptr));

  /// A spoken sentence for [tex], for a semantics label. Needs no engine.
  static String speech(String tex) => _string(tex, (b, t) => b.speech(t, nullptr));

  static String _string(String tex, Pointer<Utf8> Function(MathBindings, Pointer<Utf8>) call) {
    final b = bindings;
    final texP = tex.toNativeUtf8();
    try {
      final p = call(b, texP);
      if (p == nullptr) throw MathParseException(_lastError(b));
      final s = p.toDartString();
      b.stringFree(p);
      return s;
    } finally {
      malloc.free(texP);
    }
  }

  /// Lays out [tex] at [fontSizePx]. When [maxWidth] is given, a formula wider
  /// than that is broken into lines before relations and binary operators.
  /// With [hitTesting] on, [MathLayout.regions] records which part of the
  /// source each piece came from. Throws [MathParseException] on bad input.
  MathLayout render(String tex, double fontSizePx,
      {bool displayMode = true,
      int argb = 0xFF000000,
      Map<String, String> macros = const {},
      double? maxWidth,
      bool hitTesting = false}) {
    _check();
    final texP = tex.toNativeUtf8();
    final macroP = macros.isEmpty
        ? nullptr
        : macros.entries.map((e) => '${e.key}=${e.value}').join('\n').toNativeUtf8();
    try {
      // The C ABI takes 0xRRGGBBAA.
      final rgba = ((argb & 0x00FFFFFF) << 8) | ((argb >> 24) & 0xFF);
      final r = _b.render(_handle, texP, fontSizePx, displayMode, rgba, macroP.cast(), maxWidth ?? 0, hitTesting);
      if (r == nullptr) throw MathParseException(_lastError(_b));
      try {
        final res = r.ref;
        final items = List<MathItem>.generate(res.count, (i) {
          final it = res.items[i];
          final a = ((it.color & 0xFF) << 24) | (it.color >> 8);
          return switch (it.kind) {
            0 => MathGlyph(it.font, it.glyph, it.x, it.y, it.w, a),
            1 => MathRule(it.x, it.y, it.w, it.h, a),
            _ => MathLine(it.x, it.y, it.w, it.h, it.thickness, a),
          };
        }, growable: false);
        final regions = List<MathRegion>.generate(res.regionCount, (i) {
          final g = res.regions[i];
          return MathRegion(g.start, g.end, g.x, g.y, g.width, g.height, g.depth);
        }, growable: false);
        return MathLayout(width: res.width, ascent: res.ascent, descent: res.descent, items: items, regions: regions);
      } finally {
        _b.resultFree(r);
      }
    } finally {
      malloc.free(texP);
      if (macroP != nullptr) malloc.free(macroP);
    }
  }

  /// Outline of a glyph, cached. Null when the glyph has no outline.
  GlyphOutline? glyphOutline(int font, int glyph) {
    _check();
    return _outlines.putIfAbsent((font << 16) | glyph, () {
      final lenP = malloc<Size>();
      try {
        final p = _b.glyphOutline(_handle, font, glyph, lenP);
        if (p == nullptr) return null;
        final len = lenP.value;
        final cmds = List<double>.from(p.asTypedList(len), growable: false);
        _b.bufferFree(p, len);
        return GlyphOutline(cmds);
      } finally {
        malloc.free(lenP);
      }
    });
  }

  void _check() {
    if (_handle == nullptr) throw StateError('MathEngine is closed');
  }

  void dispose() {
    if (_handle != nullptr) {
      _b.engineFree(_handle);
      _handle = nullptr;
    }
  }
}
