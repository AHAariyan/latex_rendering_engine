import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mathcore_flutter/mathcore_flutter.dart';

/// Widget tests run on the host, against a host build of the engine
/// (`MATHCORE_LIB`, which `cargo xtask sdk flutter` sets).
void main() {
  setUpAll(() {
    final lib = Platform.environment['MATHCORE_LIB'];
    if (lib != null) MathEngine.libraryPath = lib;
  });

  Widget app(Widget child, {double scale = 1}) => MaterialApp(
        home: MediaQuery(
          data: MediaQueryData(textScaler: TextScaler.linear(scale)),
          child: Scaffold(body: Center(child: child)),
        ),
      );

  testWidgets('renders and reads the whole formula, then its parts', (tester) async {
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(app(const MathText(r'\frac{a+b}{c} = 1', fontSize: 24, speechLanguage: 'en')));
    expect(find.bySemanticsLabel('the fraction a plus b over c, equals 1'), findsOneWidget);
    expect(find.bySemanticsLabel('equals'), findsOneWidget);
    expect(find.bySemanticsLabel(RegExp('^the fraction a plus b over c')), findsWidgets);
    semantics.dispose();
  });

  testWidgets('speaks the language asked for', (tester) async {
    final semantics = tester.ensureSemantics();
    await tester.pumpWidget(app(const MathText('x^2', speechLanguage: 'fr')));
    expect(find.bySemanticsLabel('x au carré'), findsOneWidget);
    semantics.dispose();
  });

  testWidgets('follows text scaling', (tester) async {
    await tester.pumpWidget(app(const MathText('x^2', fontSize: 20)));
    final small = tester.getSize(find.byType(CustomPaint).last);
    await tester.pumpWidget(app(const MathText('x^2', fontSize: 20), scale: 2));
    final big = tester.getSize(find.byType(CustomPaint).last);
    expect(big.height, greaterThan(small.height * 1.8));
  });

  testWidgets('wraps to the width offered', (tester) async {
    const long = r'f(x) = a_0 + a_1 x + a_2 x^2 + a_3 x^3 + a_4 x^4 + a_5 x^5 + a_6 x^6';
    await tester.pumpWidget(app(const SizedBox(width: 200, child: MathText(long, fontSize: 20))));
    final size = tester.getSize(find.byType(CustomPaint).last);
    expect(size.width, lessThanOrEqualTo(200));
    expect(size.height, greaterThan(40));
  });

  testWidgets('parse errors show a message', (tester) async {
    await tester.pumpWidget(app(const MathText(r'\nosuch')));
    expect(find.textContaining('unknown command'), findsOneWidget);
  });

  testWidgets('taps report the source under the finger', (tester) async {
    MathRegion? hit;
    const tex = r'a + \frac{b}{c}';
    await tester.pumpWidget(app(MathText(tex, fontSize: 40, onTap: (r) => hit = r)));
    final box = tester.getRect(find.byType(CustomPaint).last);
    await tester.tapAt(box.centerRight - const Offset(10, 0));
    expect(hit, isNotNull);
    expect(hit!.textIn(tex), anyOf('b', 'c', r'\frac{b}{c}'));
  });
}
