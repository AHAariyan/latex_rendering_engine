//! Mutation fuzzing seeded from a real corpus.
//!
//! Random token soup (tests/robustness.rs) rarely gets deep into the parser;
//! a real formula with one brace deleted, a group duplicated or a hostile
//! token spliced in does. Every mutant goes through layout, hit testing, line
//! breaking, speech and MathML on a 256 KB stack, the smallest a host thread
//! is likely to give us, and must not panic, must keep the hit-testing
//! drawing identical, and must finish in under 50 ms.
//!
//!     cargo run --release -p mathcore --example fuzz_mutate -- formulas.txt SEED COUNT [--log FILE]
//!
//! A stack overflow aborts the process before anything can be reported, so
//! `--log` writes each input to FILE before running it: rerun the failing
//! seed with it and FILE holds the culprit.

use mathcore::{LineBreak, RenderOptions, SpeechOptions};
use std::time::{Duration, Instant};

const HOSTILE: &[&str] = &[
    "{",
    "}",
    "^",
    "_",
    "&",
    "\\\\",
    "\\frac",
    "\\sqrt[",
    "]",
    "\\left(",
    "\\right)",
    "\\middle|",
    "\\not",
    "\\over",
    "\\begin{array}{c}",
    "\\end{array}",
    "\\begin{cases}",
    "\\end{cases}",
    "\\text{",
    "\\mathrm{",
    "\\def\\x{\\x\\x}",
    "\\newcommand{\\y}[1]{#1#1}\\y",
    "\\color{red}",
    "\\displaystyle",
    "\\hspace{",
    "\\rule{1em}{",
    "\\xrightarrow[",
    "\\overset{",
    "\\stackrel",
    "\\big",
    "\\bigl.",
    "\\phantom{",
    "\\mathchoice{",
    "\\verb|",
    "'",
    "''''",
    "#",
    "%",
    "~",
    "\\",
    "\u{0}",
    "é",
    "𝑥",
    "\u{202E}",
    "\u{a0}",
    "((",
    "[[",
    "),(",
    "],[",
    "|",
    "||",
    "sqrt",
    "root(",
    "frac",
    "\"",
    "text(",
    "/",
    "^^",
    "color(",
    "abs(",
    "(:",
    ":)",
    "{:",
    ":}",
    "->",
    "oo",
];

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n.max(1) as u64) as usize
    }
}

/// A char boundary at or before `i`.
fn floor(s: &str, mut i: usize) -> usize {
    i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn mutate(rng: &mut Rng, src: &str, corpus: &[&str]) -> String {
    let mut s = src.to_string();
    for _ in 0..1 + rng.below(4) {
        let a = floor(&s, rng.below(s.len() + 1));
        let b = floor(&s, a + rng.below(16));
        s = match rng.below(6) {
            0 => format!("{}{}", &s[..a], &s[b..]),                                      // delete
            1 => format!("{}{}{}", &s[..b], &s[a..b], &s[b..]),                          // duplicate
            2 => format!("{}{}{}", &s[..a], HOSTILE[rng.below(HOSTILE.len())], &s[a..]), // hostile token
            3 => {
                // splice a piece of another formula
                let o = corpus[rng.below(corpus.len())];
                let oa = floor(o, rng.below(o.len() + 1));
                let ob = floor(o, oa + rng.below(40));
                format!("{}{}{}", &s[..a], &o[oa..ob], &s[a..])
            }
            4 => {
                // nest: wrap a range in something
                let w = ["{", "\\sqrt{", "\\frac{", "x^{", "\\left(", "\\text{"][rng.below(6)];
                let close = if w == "\\left(" { "\\right)" } else { "}" };
                format!("{}{w}{}{close}{}", &s[..a], &s[a..b], &s[b..])
            }
            _ => s.repeat(1 + rng.below(3)), // grow
        };
        if s.len() > 20_000 {
            s.truncate(floor(&s, 20_000));
        }
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let text = std::fs::read_to_string(&args[1]).expect("read corpus");
    let seed: u64 = args[2].parse().unwrap();
    let count: usize = args[3].parse().unwrap();
    let log = args.iter().position(|a| a == "--log").map(|i| args[i + 1].clone());
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    let handle = std::thread::Builder::new()
        .stack_size(256 * 1024)
        .spawn(move || run(&lines, seed, count, log))
        .unwrap();
    std::process::exit(handle.join().unwrap());
}

fn run(lines: &[String], seed: u64, count: usize, log: Option<String>) -> i32 {
    std::panic::set_hook(Box::new(|_| {}));
    let corpus: Vec<&str> = lines.iter().map(String::as_str).collect();
    let font = mathcore::bundled::font().unwrap();
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let (mut accepted, mut failures) = (0usize, 0i32);
    for _ in 0..count {
        let pick = corpus[rng.below(corpus.len())];
        let tex = mutate(&mut rng, pick, &corpus);
        if let Some(path) = &log {
            std::fs::write(path, &tex).unwrap();
        }
        let t0 = Instant::now();
        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<bool, String> {
            let opts = RenderOptions::default();
            // The same bytes read as AsciiMath must translate or fail cleanly,
            // and a translation must be TeX the engine survives.
            if let Ok(am) = mathcore::asciimath_to_tex(&tex) {
                let _ = mathcore::render(&font, &am, &opts);
            }
            let Ok(dl) = mathcore::render(&font, &tex, &opts) else {
                return Ok(false);
            };
            let hit = mathcore::render(
                &font,
                &tex,
                &RenderOptions {
                    hit_testing: true,
                    ..opts.clone()
                },
            )
            .map_err(|e| format!("hit testing rejects: {e}"))?;
            if hit.items != dl.items {
                return Err("hit testing changes the drawing".into());
            }
            let narrow = RenderOptions {
                line_break: Some(LineBreak::new(300.0)),
                ..opts.clone()
            };
            mathcore::render(&font, &tex, &narrow).map_err(|e| format!("line breaking rejects: {e}"))?;
            mathcore::render_speech_tree(&tex, &opts.macros, &SpeechOptions::default()).map_err(|e| format!("speech rejects: {e}"))?;
            mathcore::render_mathml(&tex, true, &opts.macros).map_err(|e| format!("mathml rejects: {e}"))?;
            Ok(true)
        }));
        let slow = t0.elapsed() > Duration::from_millis(50);
        let problem = match r {
            Ok(Ok(ok)) => {
                accepted += ok as usize;
                slow.then(|| format!("slow: {:?}", t0.elapsed()))
            }
            Ok(Err(e)) => Some(e),
            Err(p) => Some(format!(
                "panic: {}",
                p.downcast_ref::<String>()
                    .cloned()
                    .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                    .unwrap_or_default()
            )),
        };
        if let Some(p) = problem {
            failures += 1;
            println!("FAIL\t{p}\t{}", tex[..floor(&tex, 300)].replace('\n', " "));
        }
    }
    println!("seed {seed}: {count} mutants, {accepted} accepted, {failures} failures");
    failures.min(1)
}
