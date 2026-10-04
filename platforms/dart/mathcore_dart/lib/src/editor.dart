import 'dart:ffi';

import 'package:ffi/ffi.dart';

import 'bindings.dart';
import 'engine.dart';
import 'layout.dart';

/// A key a [MathEditor] handles.
enum MathKey {
  left('ArrowLeft'),
  right('ArrowRight'),
  up('ArrowUp'),
  down('ArrowDown'),
  home('Home'),
  end('End'),
  backspace('Backspace'),
  delete('Delete'),
  enter('Enter');

  const MathKey(this.code);
  final String code;
}

/// A rectangle of an editor layout: left, top, width, height in pixels.
class MathRect {
  const MathRect(this.x, this.y, this.width, this.height);
  final double x, y, width, height;
}

/// A [MathEditor] drawn: the formula, the caret and the selection.
class MathEditorLayout {
  const MathEditorLayout(this.formula, this.caret, this.selection);
  final MathLayout formula;
  final MathRect caret;
  final List<MathRect> selection;
}

/// The model behind a math input field: forward text, keys and taps to it,
/// lay it out, and draw the formula with the caret and selection it reports.
///
/// Typing follows the usual conventions: `/` makes a fraction of the term
/// before it, `^` and `_` open a script, `(` opens a pair, `\` starts a
/// command name, and `sqrt`, `pi`, `sin`... become what they name.
class MathEditor {
  MathEditor([String latex = '']) : _b = MathEngine.bindings {
    _handle = _b.editorNew();
    if (latex.isNotEmpty) this.latex = latex;
  }

  final MathBindings _b;
  late Pointer<MathEditorOpaque> _handle;

  void _text(void Function(Pointer<MathEditorOpaque>, Pointer<Utf8>) f, String s) {
    final p = s.toNativeUtf8();
    try {
      f(_handle, p);
    } finally {
      malloc.free(p);
    }
  }

  String _take(Pointer<Utf8> p) {
    if (p == nullptr) return '';
    final s = p.toDartString();
    _b.stringFree(p);
    return s;
  }

  /// The content as TeX. Setting it puts the cursor at the end.
  String get latex => _take(_b.editorTex(_handle));
  set latex(String tex) => _text(_b.editorSetTex, tex);

  /// The selection as TeX, empty without one.
  String get selectedLatex => _take(_b.editorSelectedTex(_handle));

  /// Types text at the cursor.
  void type(String text) => _text(_b.editorType, text);

  /// Inserts TeX as structure (a pasted `\frac{a}{b}` stays a fraction).
  void insertLatex(String tex) => _text(_b.editorInsertTex, tex);

  /// Runs a command by name: `frac`, `sqrt`, `nthroot`, `alpha`...
  void command(String name) => _text(_b.editorCommand, name);

  bool _key(String name, bool shift, bool command) {
    final p = name.toNativeUtf8();
    try {
      return _b.editorKey(_handle, p, shift, command);
    } finally {
      malloc.free(p);
    }
  }

  /// Sends a key; with [shift], arrows extend the selection.
  void key(MathKey key, {bool shift = false}) => _key(key.code, shift, false);

  void selectAll() => _key('a', false, true);
  void undo() => _key('z', false, true);
  void redo() => _key('z', true, true);

  /// Lays the editor out with [engine]'s fonts. [tap] refers to this layout.
  MathEditorLayout layout(MathEngine engine, double fontSizePx, {bool displayMode = true, int argb = 0xFF000000}) {
    final formula = engine.renderEditor(_handle, latex, fontSizePx, displayMode: displayMode, argb: argb);
    final c = malloc<Float>(4);
    final n = malloc<Size>();
    try {
      _b.editorCaret(_handle, c);
      final caret = MathRect(c[0], c[1], c[2], c[3]);
      final sel = <MathRect>[];
      final p = _b.editorSelection(_handle, n);
      if (p != nullptr) {
        for (var i = 0; i + 3 < n.value; i += 4) {
          sel.add(MathRect(p[i], p[i + 1], p[i + 2], p[i + 3]));
        }
        _b.bufferFree(p, n.value);
      }
      return MathEditorLayout(formula, caret, sel);
    } finally {
      malloc.free(c);
      malloc.free(n);
    }
  }

  /// Moves the cursor to a point of the last layout.
  void tap(double x, double y) => _b.editorTap(_handle, x, y);

  String _speech(Pointer<Utf8> Function(Pointer<MathEditorOpaque>, Pointer<Utf8>) f, String? language) {
    final p = (language ?? MathEngine.systemLanguage).toNativeUtf8();
    try {
      return _take(f(_handle, p));
    } finally {
      malloc.free(p);
    }
  }

  /// What a screen reader says for the cursor's place ("denominator, 2").
  String cursorDescription({String? language}) => _speech(_b.editorDescribe, language);

  /// The whole formula read aloud.
  String speech({String? language}) => _speech(_b.editorSpeech, language);

  /// Frees the native editor. The object must not be used afterwards.
  void dispose() {
    if (_handle != nullptr) {
      _b.editorFree(_handle);
      _handle = nullptr;
    }
  }
}
