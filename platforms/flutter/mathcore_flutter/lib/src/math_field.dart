import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/semantics.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:mathcore_dart/mathcore_dart.dart';

import 'math_painter.dart';
import 'math_text.dart';

/// Holds the formula of a [MathField] and edits it: the math counterpart of
/// a [TextEditingController]. Listeners hear every change and cursor move.
///
///     final controller = MathFieldController(latex: 'x^2');
///     MathField(controller: controller);
///     IconButton(onPressed: () => controller.command('frac'), ...);
class MathFieldController extends ChangeNotifier {
  MathFieldController({String latex = ''}) : editor = MathEditor(latex);

  /// The model, for anything this class does not wrap.
  final MathEditor editor;

  /// The formula as TeX. Setting it puts the cursor at the end.
  String get latex => editor.latex;
  set latex(String tex) {
    if (tex == editor.latex) return;
    editor.latex = tex;
    notifyListeners();
  }

  /// Types text as the keyboard would: `/` makes a fraction, `^` a superscript.
  void type(String text) => _do(() => editor.type(text));

  /// Runs a command by name (`frac`, `sqrt`, `nthroot`, `alpha`...), for toolbar buttons.
  void command(String name) => _do(() => editor.command(name));

  /// Inserts TeX as structure, as paste does.
  void insertLatex(String tex) => _do(() => editor.insertLatex(tex));

  void key(MathKey key, {bool shift = false}) => _do(() => editor.key(key, shift: shift));
  void selectAll() => _do(editor.selectAll);
  void undo() => _do(editor.undo);
  void redo() => _do(editor.redo);

  /// Moves the cursor to a point of the field's last layout.
  void tap(double x, double y) => _do(() => editor.tap(x, y));

  void _do(VoidCallback f) {
    f();
    notifyListeners();
  }

  @override
  void dispose() {
    editor.dispose();
    super.dispose();
  }
}

/// An editable formula: a text field for mathematics.
///
/// Type as you would write: `/` makes a fraction of the term before it, `^`
/// and `_` open a script, `(` opens a pair, `\alpha` or just `sqrt`, `pi`,
/// `sin` become what they name. Arrows walk into and out of structures.
/// A screen reader hears the formula and, after each key, where the cursor is.
class MathField extends StatefulWidget {
  const MathField({
    super.key,
    this.controller,
    this.onChanged,
    this.focusNode,
    this.autofocus = false,
    this.enabled = true,
    this.fontSize,
    this.color,
    this.cursorColor,
    this.hintText = '',
    this.padding = const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
    this.decoration,
    this.speechLanguage,
  });

  final MathFieldController? controller;

  /// Called with the new TeX after every change.
  final ValueChanged<String>? onChanged;
  final FocusNode? focusNode;
  final bool autofocus;
  final bool enabled;

  /// Em size in logical pixels, before text scaling; the surrounding text
  /// style's size, or 20, by default.
  final double? fontSize;
  final Color? color;
  final Color? cursorColor;

  /// Shown, dimmed, while the field is empty and unfocused.
  final String hintText;
  final EdgeInsets padding;

  /// The box behind the formula; a thin rounded border by default.
  final Decoration? decoration;

  /// The language a screen reader hears (a BCP 47 tag); null follows the app's locale.
  final String? speechLanguage;

  @override
  State<MathField> createState() => _MathFieldState();
}

/// The text the platform keyboard edits: one invisible character, so that
/// Backspace always has something to delete and reports it.
const _sentinel = '​';
const _resting = TextEditingValue(text: _sentinel, selection: TextSelection.collapsed(offset: 1));

class _MathFieldState extends State<MathField> with TextInputClient {
  MathFieldController? _own;
  FocusNode? _ownFocus;
  TextInputConnection? _connection;
  MathEditorLayout? _layout;
  String _lastLatex = '';
  Timer? _blink;
  final ValueNotifier<bool> _caretOn = ValueNotifier(true);

  MathFieldController get _controller => widget.controller ?? (_own ??= MathFieldController());
  FocusNode get _focus => widget.focusNode ?? (_ownFocus ??= FocusNode());

  @override
  void initState() {
    super.initState();
    _lastLatex = _controller.latex;
    _controller.addListener(_changed);
    _focus.addListener(_focusChanged);
  }

  @override
  void didUpdateWidget(MathField old) {
    super.didUpdateWidget(old);
    if (old.controller != widget.controller) {
      (old.controller ?? _own)?.removeListener(_changed);
      _controller.addListener(_changed);
      _lastLatex = _controller.latex;
    }
    if (old.focusNode != widget.focusNode) {
      (old.focusNode ?? _ownFocus)?.removeListener(_focusChanged);
      _focus.addListener(_focusChanged);
    }
  }

  @override
  void dispose() {
    _controller.removeListener(_changed);
    _focus.removeListener(_focusChanged);
    _connection?.close();
    _blink?.cancel();
    _own?.dispose();
    _ownFocus?.dispose();
    _caretOn.dispose();
    super.dispose();
  }

  String? get _language => widget.speechLanguage ?? Localizations.maybeLocaleOf(context)?.toLanguageTag();

  void _changed() {
    if (!mounted) return;
    final tex = _controller.latex;
    final edited = tex != _lastLatex;
    _lastLatex = tex;
    setState(() {});
    _restartBlink();
    if (edited) widget.onChanged?.call(tex);
    if (_focus.hasFocus) {
      // sendAnnouncement replaces this from Flutter 3.35; announce works on both.
      // ignore: deprecated_member_use
      SemanticsService.announce(_controller.editor.cursorDescription(language: _language), TextDirection.ltr);
    }
  }

  void _focusChanged() {
    if (_focus.hasFocus && widget.enabled) {
      _openConnection();
    } else {
      _connection?.close();
      _connection = null;
    }
    _restartBlink();
    setState(() {});
  }

  void _openConnection() {
    if (_connection?.attached ?? false) {
      _connection!.show();
      return;
    }
    _connection = TextInput.attach(
      this,
      const TextInputConfiguration(
        inputType: TextInputType.visiblePassword,
        autocorrect: false,
        enableSuggestions: false,
        enableIMEPersonalizedLearning: false,
        inputAction: TextInputAction.done,
      ),
    )
      ..setEditingState(_resting)
      ..show();
  }

  void _restartBlink() {
    _blink?.cancel();
    _caretOn.value = true;
    if (_focus.hasFocus && widget.enabled) {
      _blink = Timer.periodic(const Duration(milliseconds: 530), (_) => _caretOn.value = !_caretOn.value);
    }
  }

  // ---- TextInputClient: the soft keyboard ----

  @override
  void updateEditingValue(TextEditingValue value) {
    if (!widget.enabled) return;
    // While the keyboard is composing (an IME), wait for the committed text.
    if (value.composing.isValid && !value.composing.isCollapsed) return;
    final text = value.text;
    if (text.length < _sentinel.length || !text.startsWith(_sentinel)) {
      _controller.key(MathKey.backspace);
    } else if (text.length > _sentinel.length) {
      _controller.type(text.substring(_sentinel.length));
    }
    if (text != _sentinel) _connection?.setEditingState(_resting);
  }

  @override
  void performAction(TextInputAction action) => _controller.key(MathKey.enter);

  @override
  TextEditingValue? get currentTextEditingValue => _resting;

  @override
  AutofillScope? get currentAutofillScope => null;

  @override
  void connectionClosed() => _connection = null;

  @override
  void performPrivateCommand(String action, Map<String, dynamic> data) {}

  @override
  void showAutocorrectionPromptRect(int start, int end) {}

  @override
  void updateFloatingCursor(RawFloatingCursorPoint point) {}

  // ---- hardware keys ----

  KeyEventResult _onKey(FocusNode node, KeyEvent event) {
    if (event is KeyUpEvent) return KeyEventResult.ignored;
    final keys = HardwareKeyboard.instance;
    final shift = keys.isShiftPressed;
    final command = keys.isControlPressed || keys.isMetaPressed;
    final k = event.logicalKey;
    final c = _controller;
    if (command) {
      if (k == LogicalKeyboardKey.keyA) {
        c.selectAll();
      } else if (k == LogicalKeyboardKey.keyC || k == LogicalKeyboardKey.keyX) {
        final tex = c.editor.selectedLatex.isEmpty ? c.latex : c.editor.selectedLatex;
        Clipboard.setData(ClipboardData(text: tex));
        if (k == LogicalKeyboardKey.keyX && widget.enabled && c.editor.selectedLatex.isNotEmpty) c.key(MathKey.backspace);
      } else if (!widget.enabled) {
        return KeyEventResult.ignored;
      } else if (k == LogicalKeyboardKey.keyZ) {
        shift ? c.redo() : c.undo();
      } else if (k == LogicalKeyboardKey.keyY) {
        c.redo();
      } else if (k == LogicalKeyboardKey.keyV) {
        Clipboard.getData(Clipboard.kTextPlain).then((d) {
          final text = d?.text ?? '';
          if (text.isNotEmpty && mounted) c.insertLatex(text);
        });
      } else {
        return KeyEventResult.ignored;
      }
      return KeyEventResult.handled;
    }
    final moves = <LogicalKeyboardKey, MathKey>{
      LogicalKeyboardKey.arrowLeft: MathKey.left,
      LogicalKeyboardKey.arrowRight: MathKey.right,
      LogicalKeyboardKey.arrowUp: MathKey.up,
      LogicalKeyboardKey.arrowDown: MathKey.down,
      LogicalKeyboardKey.home: MathKey.home,
      LogicalKeyboardKey.end: MathKey.end,
    };
    final edits = <LogicalKeyboardKey, MathKey>{
      LogicalKeyboardKey.backspace: MathKey.backspace,
      LogicalKeyboardKey.delete: MathKey.delete,
      LogicalKeyboardKey.enter: MathKey.enter,
      LogicalKeyboardKey.numpadEnter: MathKey.enter,
    };
    if (moves.containsKey(k)) {
      c.key(moves[k]!, shift: shift);
      return KeyEventResult.handled;
    }
    if (edits.containsKey(k)) {
      if (widget.enabled) c.key(edits[k]!);
      return KeyEventResult.handled;
    }
    return KeyEventResult.ignored;
  }

  // ---- build ----

  @override
  Widget build(BuildContext context) {
    final style = DefaultTextStyle.of(context).style;
    final size = MediaQuery.textScalerOf(context).scale(widget.fontSize ?? style.fontSize ?? 20);
    final color = widget.color ?? style.color ?? const Color(0xFF000000);
    final accent = widget.cursorColor ?? const Color(0xFF1A73E8);
    final focused = _focus.hasFocus;
    MathEditorLayout? layout;
    try {
      layout = _controller.editor.layout(MathCore.engine, size, argb: color.toARGB32());
    } on MathParseException {
      layout = _layout;
    }
    _layout = layout;
    final w = (layout?.formula.width ?? 0).clamp(size * 2, double.infinity).toDouble();
    final h = (layout?.formula.height ?? 0).clamp(size * 1.2, double.infinity).toDouble();
    final empty = _controller.latex.isEmpty;
    final speech = _controller.editor.speech(language: _language);

    return Semantics(
      textField: true,
      focused: focused,
      enabled: widget.enabled,
      label: widget.hintText.isEmpty ? 'math' : widget.hintText,
      value: speech,
      onTap: _focus.requestFocus,
      child: Focus(
        focusNode: _focus,
        autofocus: widget.autofocus,
        onKeyEvent: _onKey,
        child: GestureDetector(
          behavior: HitTestBehavior.opaque,
          onTapDown: (d) {
            _focus.requestFocus();
            if (focused) _openConnection();
            final dy = (h - (layout?.formula.height ?? h)) / 2;
            _controller.tap(d.localPosition.dx - widget.padding.left, d.localPosition.dy - widget.padding.top - dy);
          },
          child: DecoratedBox(
            decoration: widget.decoration ??
                BoxDecoration(
                  border: Border.all(color: focused ? accent : color.withValues(alpha: 0.35), width: focused ? 2 : 1),
                  borderRadius: BorderRadius.circular(6),
                ),
            child: Padding(
              padding: widget.padding,
              child: ExcludeSemantics(
                child: SizedBox(
                  width: w,
                  height: h,
                  child: empty && !focused && widget.hintText.isNotEmpty
                      ? Align(
                          alignment: Alignment.centerLeft,
                          child: Text(widget.hintText,
                              maxLines: 1,
                              softWrap: false,
                              overflow: TextOverflow.visible,
                              style: style.copyWith(fontSize: size * 0.8, color: color.withValues(alpha: 0.45))),
                        )
                      : layout == null
                          ? null
                          : CustomPaint(
                              painter: _FieldPainter(layout, MathCore.cache, accent, focused && widget.enabled, _caretOn, h),
                            ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _FieldPainter extends CustomPainter {
  _FieldPainter(this.layout, this.cache, this.accent, this.showCaret, this.caretOn, this.height) : super(repaint: caretOn);

  final MathEditorLayout layout;
  final GlyphPathCache cache;
  final Color accent;
  final bool showCaret;
  final ValueListenable<bool> caretOn;
  final double height;

  @override
  void paint(Canvas canvas, Size size) {
    final o = Offset(0, (height - layout.formula.height) / 2);
    final paint = Paint()..color = accent.withValues(alpha: 0.3);
    for (final r in layout.selection) {
      canvas.drawRect(Rect.fromLTWH(o.dx + r.x, o.dy + r.y, r.width, r.height), paint);
    }
    MathPainter.draw(canvas, layout.formula, cache, o);
    if (showCaret && caretOn.value) {
      final c = layout.caret;
      canvas.drawRect(Rect.fromLTWH(o.dx + c.x, o.dy + c.y, c.width < 2 ? 2 : c.width, c.height), Paint()..color = accent);
    }
  }

  @override
  bool shouldRepaint(_FieldPainter old) =>
      old.layout != layout || old.showCaret != showCaret || old.accent != accent || old.height != height;
}
