import 'package:mathcore_dart/mathcore_dart.dart';
import 'package:test/test.dart';

import 'support.dart';

void main() {
  setUpAll(() => MathEngine.libraryPath = libraryForTests());

  test('speech in other languages and braille', () {
    expect(MathEngine.speechWith('x^2', SpeechVerbosity.brief, language: 'pt-BR'), 'x ao quadrado');
    expect(MathEngine.speechWith('x^2', SpeechVerbosity.brief, language: 'hi'), 'x का वर्ग');
    expect(MathEngine.nemeth('x^2'), '⠭⠘⠆');
  });

  test('35 speech languages, word order per language', () {
    expect(MathEngine.speechLanguages, hasLength(35));
    expect(MathEngine.speechWith(r'\frac{a}{b}', SpeechVerbosity.brief, language: 'ja'), 'b分のa');
    expect(MathEngine.speechWith(r'\frac{a}{b}', SpeechVerbosity.brief, language: 'zh-TW'), contains('分之'));
  });

  test('text in other scripts draws with system fonts', () {
    const tex = r'x = \text{বাংলা 你好}';
    final engine = MathEngine.bundled()..usesSystemFonts = false;
    expect(engine.missingCharacters(tex), isNotEmpty);
    engine.usesSystemFonts = true;
    engine.render(tex, 32);
    // Machines without these scripts' fonts (a bare CI image) skip the rest.
    if (engine.missingCharacters(tex).isNotEmpty) return;
    expect(engine.render(tex, 32).items.whereType<MathGlyph>().any((g) => g.font > 1), isTrue);
  });

  test('editor: typing, keys, caret, selection, speech', () {
    final e = MathEditor()..type('x^2');
    e.key(MathKey.right);
    e.type('+1/2');
    expect(e.latex, r'x^{2}+\frac{1}{2}');
    expect(e.cursorDescription(language: 'en'), 'denominator, 2');
    final engine = MathEngine.bundled();
    final l = e.layout(engine, 32);
    expect(l.caret.height, greaterThan(5));
    expect(l.caret.y, greaterThan(l.formula.ascent / 2));
    e.selectAll();
    expect(e.layout(engine, 32).selection, isNotEmpty);
    expect(e.selectedLatex, e.latex);
    e.key(MathKey.backspace);
    expect(e.latex, '');
    e.undo();
    e.tap(0, 10);
    e.insertLatex(r'\sqrt{y}');
    expect(e.latex, startsWith(r'\sqrt{y}'));
    e.dispose();
  });

  test('speech verbosity and tree', () {
    expect(MathEngine.speechWith(r'\frac{1}{2}', SpeechVerbosity.verbose, language: 'en'), 'the fraction 1 over 2, end fraction');
    final tree = MathEngine.speechTree(r'\frac{a+b}{c} = 1', language: 'en');
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
