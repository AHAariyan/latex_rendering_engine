// Checks that the dart:ffi binding lays out every corpus formula exactly as the
// Rust core does. Run through scripts/parity.sh.
import 'dart:convert';
import 'dart:io';

import 'package:mathcore_dart/mathcore_dart.dart';
import 'package:test/test.dart';

void main() {
  test('layout matches the Rust core for every corpus formula', () {
    MathEngine.libraryPath = Platform.environment['MATHCORE_LIB'] ?? '../../../target/release/libmathcore_ffi.so';
    final file = File(Platform.environment['MATHCORE_PARITY'] ?? '../../../tests/parity/expected.json');
    final expected = jsonDecode(file.readAsStringSync()) as Map<String, dynamic>;
    final fontSize = (expected['fontSize'] as num).toDouble();
    final engine = MathEngine.bundled();
    const tol = 0.002; // the reference is rounded to three decimals
    double round(double v) => (v * 1000).roundToDouble() / 1000;

    var count = 0;
    for (final c in expected['cases'] as List) {
      final name = c['name'] as String;
      final width = (c['maxWidth'] as num).toDouble();
      final layout = engine.render(
        c['tex'] as String,
        fontSize,
        displayMode: c['display'] as bool,
        maxWidth: width > 0 ? width : null,
      );
      expect(round(layout.width), closeTo((c['width'] as num).toDouble(), tol), reason: '$name width');
      expect(round(layout.ascent), closeTo((c['ascent'] as num).toDouble(), tol), reason: '$name ascent');
      expect(round(layout.descent), closeTo((c['descent'] as num).toDouble(), tol), reason: '$name descent');
      final items = c['items'] as List;
      expect(layout.items.length, items.length, reason: '$name item count');
      for (var i = 0; i < items.length; i++) {
        final e = (items[i] as List).map((v) => (v as num).toDouble()).toList();
        final got = layout.items[i];
        final actual = switch (got) {
          MathGlyph g => [0.0, g.id.toDouble(), g.x, g.y, g.emSize, g.font.toDouble(), 0.0],
          MathRule r => [1.0, 0.0, r.x, r.y, r.width, r.height, 0.0],
          MathLine l => [2.0, 0.0, l.x1, l.y1, l.x2, l.y2, l.thickness],
        };
        for (var f = 0; f < 7; f++) {
          expect(round(actual[f]), closeTo(e[f], tol), reason: '$name item $i field $f');
        }
        expect(got.argb, e[7].toInt(), reason: '$name item $i color');
      }
      count += items.length;
    }
    engine.dispose();
    expect(count, greaterThan(500));
  });
}
