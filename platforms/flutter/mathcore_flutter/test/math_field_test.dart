import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mathcore_flutter/mathcore_flutter.dart';

/// MathField driven the way the platform drives a text field: the text
/// input channel for the soft keyboard, key events for a hardware one, taps.
void main() {
  setUpAll(() {
    final lib = Platform.environment['MATHCORE_LIB'];
    if (lib != null) MathEngine.libraryPath = lib;
  });

  Widget app(Widget child) => MaterialApp(home: Scaffold(body: Center(child: child)));

  /// What the soft keyboard sends when a character is typed after the sentinel.
  Future<void> softType(WidgetTester tester, String text) async {
    for (final ch in text.split('')) {
      tester.testTextInput.updateEditingValue(TextEditingValue(text: '​$ch', selection: const TextSelection.collapsed(offset: 2)));
      await tester.pump();
    }
  }

  testWidgets('the soft keyboard builds a formula and backspace deletes', (tester) async {
    final controller = MathFieldController();
    final changes = <String>[];
    await tester.pumpWidget(app(MathField(controller: controller, onChanged: changes.add, hintText: 'answer')));
    await tester.tap(find.byType(MathField));
    await tester.pump();
    expect(tester.testTextInput.isVisible, isTrue);
    await softType(tester, '1/2');
    expect(controller.latex, r'\frac{1}{2}');
    expect(changes.last, r'\frac{1}{2}');
    // Backspace: the keyboard deletes the sentinel.
    tester.testTextInput.updateEditingValue(const TextEditingValue(text: '', selection: TextSelection.collapsed(offset: 0)));
    await tester.pump();
    expect(controller.latex, r'\frac{1}{}');
    await softType(tester, '3');
    await tester.testTextInput.receiveAction(TextInputAction.done);
    await softType(tester, '+x');
    expect(controller.latex, r'\frac{1}{3}+x');
  });

  testWidgets('hardware keys move, select and undo', (tester) async {
    final controller = MathFieldController(latex: 'ab');
    await tester.pumpWidget(app(MathField(controller: controller, autofocus: true)));
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
    await softType(tester, 'x');
    expect(controller.latex, 'axb');
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    expect(controller.latex, 'ab');
    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    await tester.sendKeyEvent(LogicalKeyboardKey.delete);
    expect(controller.latex, '');
    await tester.sendKeyDownEvent(LogicalKeyboardKey.control);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.control);
    expect(controller.latex, 'ab');
  });

  testWidgets('sizes to the formula, speaks it, and a tap places the cursor', (tester) async {
    final semantics = tester.ensureSemantics();
    final controller = MathFieldController();
    await tester.pumpWidget(app(MathField(controller: controller, fontSize: 30, speechLanguage: 'en', hintText: 'answer')));
    final empty = tester.getSize(find.byType(MathField));
    controller.latex = r'\frac{a+b}{c}+\sqrt{x}';
    await tester.pump();
    final size = tester.getSize(find.byType(MathField));
    expect(size.width, greaterThan(empty.width));
    expect(size.height, greaterThan(empty.height));
    expect(tester.getSemantics(find.byType(MathField)).value, 'the fraction a plus b over c, plus the square root of x');
    // Tap at the left edge: the cursor goes to the start.
    await tester.tapAt(tester.getTopLeft(find.byType(MathField)) + Offset(11, size.height / 2));
    await tester.pump();
    controller.type('y');
    expect(controller.latex, startsWith('y'));
    semantics.dispose();
  });

  testWidgets('a disabled field ignores input; toolbar commands work', (tester) async {
    final controller = MathFieldController(latex: 'x');
    await tester.pumpWidget(app(MathField(controller: controller, enabled: false, autofocus: true)));
    await tester.pump();
    await tester.sendKeyEvent(LogicalKeyboardKey.backspace);
    expect(controller.latex, 'x');
    controller.command('sqrt');
    controller.type('2');
    expect(controller.latex, r'x\sqrt{2}');
  });
}
