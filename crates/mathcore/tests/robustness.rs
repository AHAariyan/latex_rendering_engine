//! Deterministic fuzzing: the engine must never panic, whatever the input.
//! A panic inside the FFI layer aborts the host application, so this test is
//! the last line of defense for every binding.

use mathcore::{MathFont, RenderOptions};

const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");

/// xorshift64* — tiny, deterministic, good enough for input generation.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn pick<'a>(&mut self, items: &[&'a str]) -> &'a str {
        items[(self.next() % items.len() as u64) as usize]
    }
}

const FRAGMENTS: &[&str] = &[
    "x",
    "y",
    "2",
    "+",
    "-",
    "=",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    "^",
    "_",
    "'",
    "&",
    "\\\\",
    " ",
    ",",
    "|",
    "~",
    "#",
    "%",
    "$",
    "\\frac",
    "\\sqrt",
    "\\sqrt[",
    "\\left(",
    "\\right)",
    "\\left.",
    "\\right.",
    "\\left",
    "\\right",
    "\\middle|",
    "\\sum",
    "\\int",
    "\\lim",
    "\\sin",
    "\\alpha",
    "\\infty",
    "\\to",
    "\\cdot",
    "\\pm",
    "\\hat",
    "\\widehat",
    "\\vec",
    "\\overline",
    "\\underline",
    "\\mathbf",
    "\\mathbb",
    "\\text{ab}",
    "\\text{",
    "\\operatorname{f}",
    "\\displaystyle",
    "\\scriptstyle",
    "\\big(",
    "\\Bigg]",
    "\\binom",
    "\\over",
    "\\choose",
    "\\atop",
    "\\genfrac",
    "\\begin{pmatrix}",
    "\\end{pmatrix}",
    "\\begin{cases}",
    "\\end{cases}",
    "\\begin{array}{|c|}",
    "\\end{array}",
    "\\hline",
    "\\\\[2pt]",
    "\\begin{aligned}",
    "\\end{aligned}",
    "\\substack{",
    "\\overset",
    "\\underset",
    "\\color{red}",
    "\\textcolor{#0f0}",
    "\\boxed",
    "\\cancel",
    "\\underbrace",
    "\\xrightarrow",
    "\\xrightarrow[",
    "\\pmod",
    "\\bmod",
    "\\mathop",
    "\\limits",
    "\\nolimits",
    "\\phantom",
    "\\vphantom",
    "\\hspace{1em}",
    "\\kern2pt",
    "\\kern",
    "\\not",
    "\\not=",
    "\\newcommand",
    "\\newcommand{\\f}[1]{#1#1}",
    "\\f",
    "\\def\\g#1#2{#2#1}",
    "\\g",
    "\\foo",
    "\\",
    "\\{",
    "\\}",
    "\\,",
    "\\quad",
    "α",
    "∑",
    "≤",
    "→",
    "𝑥",
    "\u{0}",
    "\u{FFFD}",
    "⏟",
    "√",
    "\\varGamma",
    "\\rm",
    "\\bf",
    "\\limits^",
    "\\stackrel",
];

fn generate(rng: &mut Rng) -> String {
    let n = 1 + (rng.next() % 24) as usize;
    let mut s = String::new();
    for _ in 0..n {
        s.push_str(rng.pick(FRAGMENTS));
    }
    s
}

#[test]
fn random_inputs_never_panic() {
    let font = MathFont::from_bytes(FONT).unwrap();
    let opts = RenderOptions::default();
    let inline = RenderOptions {
        display_mode: false,
        font_size: 7.0,
        ..Default::default()
    };
    let mut rng = Rng(0x9E3779B97F4A7C15);
    let mut ok = 0usize;
    for i in 0..20_000 {
        let tex = generate(&mut rng);
        let r = std::panic::catch_unwind(|| {
            let _ = mathcore::render(&font, &tex, &opts);
            let _ = mathcore::render(&font, &tex, &inline);
        });
        assert!(r.is_ok(), "panic on input #{i}: {tex:?}");
        if mathcore::render(&font, &tex, &opts).is_ok() {
            ok += 1;
        }
    }
    // Sanity: the generator produces a healthy mix of valid and invalid input.
    assert!(ok > 500, "only {ok} valid inputs; generator too hostile");
}

#[test]
fn pathological_inputs_never_panic() {
    // Host threads on Android and in wasm can have stacks under 1 MB. A release
    // build handles the 64-level nesting limit within 256 KB (measured); debug
    // frames are several times larger, so the budget depends on the profile.
    let stack = if cfg!(debug_assertions) { 4 * 1024 * 1024 } else { 256 * 1024 };
    let handle = std::thread::Builder::new().stack_size(stack).spawn(pathological_inputs).unwrap();
    handle.join().expect("pathological input overflowed the stack or panicked");
}

fn pathological_inputs() {
    let font = MathFont::from_bytes(FONT).unwrap();
    let opts = RenderOptions::default();
    let deep_braces = "{".repeat(5_000) + "x" + &"}".repeat(5_000);
    let deep_bare_frac = "\\frac".repeat(2_000) + "xy";
    let deep_bare_sqrt = "\\sqrt".repeat(2_000) + "x";
    let deep_hat = "\\hat".repeat(2_000) + "x";
    let deep_frac = "\\frac{".repeat(300) + "x" + &"}{y}".repeat(300);
    let deep_sup = "x^{".repeat(400) + "1" + &"}".repeat(400);
    let deep_sqrt = "\\sqrt{".repeat(400) + "x" + &"}".repeat(400);
    let deep_left = "\\left(".repeat(400) + "x" + &"\\right)".repeat(400);
    let long_row = "x+".repeat(20_000) + "y";
    let huge_matrix = format!("\\begin{{matrix}}{}\\end{{matrix}}", "a&b&c\\\\".repeat(300));
    let macro_bomb = "\\def\\a{xx}\\def\\b{\\a\\a}\\def\\c{\\b\\b}\\def\\d{\\c\\c}\\def\\e{\\d\\d}\\def\\f{\\e\\e}\\f\\f\\f";
    let unterminated_text = "\\text{".to_string() + &"a".repeat(10_000);
    let cases: Vec<String> = vec![
        deep_braces,
        deep_bare_frac,
        deep_bare_sqrt,
        deep_hat,
        deep_frac,
        deep_sup,
        deep_sqrt,
        deep_left,
        long_row,
        huge_matrix,
        macro_bomb.into(),
        unterminated_text,
        "\\\\".repeat(1000),
        "^".repeat(100),
        "\\sqrt[".repeat(100),
        "\\begin{".repeat(50),
        "\\end{x}".repeat(50),
        "\\kern".to_string(),
        "\\kern-".to_string(),
        "\\hspace{}".to_string(),
        "\\genfrac{}{}{}{}{}{}".to_string(),
        "\\big".to_string(),
        "\\left".to_string(),
        "\\color{}".to_string(),
        "\\newcommand".to_string(),
        "\\def".to_string(),
        "\\def\\x".to_string(),
        "\\newcommand{\\x}[99]{#99}\\x".to_string(),
        "#1".to_string(),
        "\\text{\\}".to_string(),
    ];
    for tex in &cases {
        let r = std::panic::catch_unwind(|| {
            let _ = mathcore::render(&font, tex, &opts);
        });
        assert!(r.is_ok(), "panic on {:?}", &tex[..tex.len().min(80)]);
    }
}
