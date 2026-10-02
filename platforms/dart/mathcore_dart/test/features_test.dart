import 'package:mathcore_dart/mathcore_dart.dart';
import 'package:test/test.dart';

import 'support.dart';

void main() {
  setUpAll(() => MathEngine.libraryPath = libraryForTests());

  test('speech verbosity and tree', () {
    expect(MathEngine.speechWith(r'\frac{1}{2}', SpeechVerbosity.verbose), 'the fraction 1 over 2, end fraction');
    final tree = MathEngine.speechTree(r'\frac{a+b}{c} = 1');
    expect(tree.role, 'formula');
    expect(tree.children.first.children.first.announcement, 'numerator: a plus b');
  });

  test('hit testing maps back to non-ASCII source', () {
    const tex = r'α ≤ \frac{a}{b}';
    final layout = MathEngine.bundled().render(tex, 32, hitTesting: true);
    final frac = layout.regions.reduce((a, b) => a.area >= b.area ? a : b);
    expect(frac.textIn(tex), r'\frac{a}{b}');
    expect(layout.highlight(frac.start, frac.end), hasLength(1));
  });

  test('asciimath, chemistry, budget and cache', () {
    expect(MathEngine.asciimathToTex('x/y'), r'\frac{x}{y}');
    final e = MathEngine.bundled();
    expect(e.render(r'\ce{2H2 + O2 -> 2H2O}', 20).width, greaterThan(50));
    e.setBudget(maxNodes: 10);
    expect(() => e.render('${'x+' * 50}x', 20), throwsA(isA<MathParseException>()));
    e.setBudget();
    e.setCacheCapacity(0);
    expect(e.render('${'x+' * 50}x', 20).width, greaterThan(0));
  });
}
