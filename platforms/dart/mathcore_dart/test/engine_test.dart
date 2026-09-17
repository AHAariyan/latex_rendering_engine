import 'dart:io';

import 'package:mathcore_dart/mathcore_dart.dart';
import 'package:test/test.dart';

void main() {
  setUpAll(() {
    final env = Platform.environment['MATHCORE_LIB'];
    MathEngine.libraryPath = env ?? '../../../target/release/libmathcore_ffi.so';
  });

  test('bundled engine renders a fraction', () {
    final e = MathEngine.bundled();
    expect(e.unitsPerEm, 1000);
    final l = e.render(r'\frac{a}{b} + x^2', 32);
    expect(l.width, greaterThan(0));
    expect(l.items.whereType<MathRule>().length, 1);
    expect(l.items.whereType<MathGlyph>().length, 5);
    expect(l.items.first.argb, 0xFF000000);
    e.dispose();
  });

  test('colors, macros and errors', () {
    final e = MathEngine.bundled();
    final l = e.render(r'\half', 20, argb: 0xFFFF0000, macros: {r'\half': r'\frac{1}{2}'});
    expect(l.items.whereType<MathRule>().length, 1);
    expect(l.items.first.argb, 0xFFFF0000);
    expect(() => e.render(r'\frac{a', 20), throwsA(isA<MathParseException>().having((x) => x.message, 'message', contains('parse error'))));
    e.dispose();
  });

  test('line breaking fits the requested width', () {
    final e = MathEngine.bundled();
    const tex = r'a + b + c + d + e + f + g + h + i + j';
    final wide = e.render(tex, 32);
    final narrow = e.render(tex, 32, maxWidth: 150);
    expect(narrow.width, lessThanOrEqualTo(150));
    expect(narrow.width, lessThan(wide.width));
    expect(narrow.height, greaterThan(wide.height));
    e.dispose();
  });

  test('accessibility output', () {
    expect(MathEngine.speech(r'x^2 + \frac{1}{2}'), 'x squared plus 1 over 2');
    final ml = MathEngine.mathml(r'x^2');
    expect(ml, startsWith('<math'));
    expect(ml, contains('<msup>'));
    expect(() => MathEngine.speech(r'\frac{a'), throwsA(isA<MathParseException>()));
  });

  test('glyph outlines are cached command streams', () {
    final e = MathEngine.bundled();
    final g = e.render('x', 32).items.first as MathGlyph;
    final o = e.glyphOutline(g.id)!;
    expect(o.commands.first, 0);
    expect(identical(o, e.glyphOutline(g.id)), isTrue);
    expect(e.glyphOutline(0), isNull);
    e.dispose();
  });

  test('custom font bytes', () {
    final bytes = File('../../../assets/fonts/latinmodern-math.otf').readAsBytesSync();
    final e = MathEngine.fromFont(bytes);
    expect(e.render(r'\sqrt{2}', 16).items, isNotEmpty);
    e.dispose();
    expect(() => MathEngine.fromFont(bytes.sublist(0, 10)), throwsStateError);
  });
}
