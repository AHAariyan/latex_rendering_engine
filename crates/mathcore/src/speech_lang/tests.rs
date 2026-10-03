use super::*;
use crate::a11y::{speech_with, SpeechOptions, Verbosity};
use std::cell::RefCell;
use std::collections::BTreeSet;

thread_local! {
    static USED: RefCell<BTreeSet<String>> = const { RefCell::new(BTreeSet::new()) };
}

pub(super) fn record(english: &str) {
    USED.with(|u| u.borrow_mut().insert(english.to_string()));
}

fn say(tex: &str, lang: &str) -> String {
    let opts = SpeechOptions {
        language: Language::from_tag(lang),
        verbosity: Verbosity::Brief,
    };
    speech_with(&crate::parse(tex).unwrap(), &opts)
}

fn slots(s: &str) -> Vec<usize> {
    let mut v: Vec<usize> = (0..10).filter(|i| s.contains(&format!("{{{i}}}"))).collect();
    v.sort();
    v
}

/// Formulas that exercise every construct the writer reads.
const SAMPLES: &[&str] = &[
    r"x^2 + y^2 = z^2",
    r"\frac{a+b}{c} \ne \frac{1}{2}",
    r"\sqrt{x} + \sqrt[3]{y} + \sqrt[n]{z}",
    r"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}",
    r"\int_0^\infty e^{-x^2}\,dx",
    r"\lim_{x \to 0} \frac{\sin x}{x} = 1",
    r"\max_{x} f(x) \le \left| g(x) \right|",
    r"\binom{n}{k} \left\lfloor x \right\rfloor \left\lceil y \right\rceil \left\| v \right\|",
    r"\hat{x} \tilde{y} \bar{z} \vec{v} \dot{a} \ddot{b} \underline{c}",
    r"f'(x) + f''(x) + 90^\circ",
    r"\begin{pmatrix} a & b \\ c & \end{pmatrix}",
    r"A \subseteq B \cup C, \; \forall \epsilon > 0 \; \exists \delta",
    r"\Gamma(n) = (n-1)! \quad \alpha \beta \Omega",
    r"x_1^{n+1} \to \infty \implies a \equiv b \pmod{n}",
    r"\xrightarrow[b]{a} \overbrace{x+y} \boxed{E=mc^2} \cancel{x}",
    r"\cos^2\theta + \log_2 8 + \ln e + \det A",
    r"x \tag{2}",
    r"x < y, \; a > b, \; p \le q, \; r \ge s",
    r"a \in A, \; b \notin B, \; A \subset B, \; C \supseteq D",
    r"f: A \to B, \; x \mapsto x^2",
    r"p \implies q, \; q \impliedby p, \; p \iff q",
    r"a \mid b, \; \ell \parallel m, \; \ell \perp n, \; a \approx b",
    r"\log_2 8 = 3, \; \cos^2\theta + \sin^2\theta = 1",
    r"\frac{\partial f}{\partial x} + \nabla \cdot \vec{E} = \frac{\rho}{\varepsilon_0}",
];

#[test]
fn every_language_translates_every_phrase() {
    let keys: BTreeSet<&str> = KEYS.iter().map(|(k, _)| *k).collect();
    for lang in Language::ALL {
        let Some(table) = lang.table() else { continue };
        let mut seen = BTreeSet::new();
        for (k, t) in table {
            assert!(keys.contains(k), "{}: `{k}` is not a phrase the writer uses", lang.tag());
            assert!(seen.insert(*k), "{}: `{k}` translated twice", lang.tag());
            assert!(!t.trim().is_empty(), "{}: `{k}` is empty", lang.tag());
            assert_eq!(slots(k), slots(t), "{}: `{k}` -> `{t}` must keep the slots", lang.tag());
            for i in slots(t) {
                assert_eq!(t.matches(&format!("{{{i}}}")).count(), 1, "{}: `{t}` repeats a slot", lang.tag());
            }
        }
        let missing: Vec<_> = keys.iter().filter(|k| !seen.contains(*k)).collect();
        assert!(
            missing.is_empty(),
            "{} is missing {} phrases: {missing:?}",
            lang.tag(),
            missing.len()
        );
    }
}

#[test]
fn every_phrase_the_writer_uses_is_listed() {
    let keys: BTreeSet<&str> = KEYS.iter().map(|(k, _)| *k).collect();
    USED.with(|u| u.borrow_mut().clear());
    for tex in SAMPLES {
        let nodes = crate::parse(tex).unwrap();
        for verbosity in [Verbosity::Verbose, Verbosity::Brief, Verbosity::Superbrief] {
            let opts = SpeechOptions {
                verbosity,
                language: Language::Spanish,
            };
            speech_with(&nodes, &opts);
            crate::a11y::speech_tree(&nodes, &opts);
        }
    }
    let used = USED.with(|u| u.borrow().clone());
    // Letters, digits and punctuation pass through as themselves.
    let unlisted: Vec<_> = used
        .iter()
        .filter(|u| !keys.contains(u.as_str()) && !keys.contains(format!("the {u}").as_str()))
        .filter(|u| u.chars().count() > 1 && !u.starts_with("capital "))
        .collect();
    assert!(unlisted.is_empty(), "phrases missing from keys.rs: {unlisted:?}");
}

#[test]
fn tags_round_trip_and_regions_pick_the_script() {
    for lang in Language::ALL {
        assert_eq!(Language::from_tag(lang.tag()), lang, "{}", lang.tag());
        assert!(!lang.native_name().is_empty());
    }
    assert_eq!(Language::from_tag("es-MX"), Language::Spanish);
    assert_eq!(Language::from_tag("pt_BR"), Language::Portuguese);
    assert_eq!(Language::from_tag("bn-BD"), Language::Bengali);
    assert_eq!(Language::from_tag("zh"), Language::ChineseSimplified);
    assert_eq!(Language::from_tag("zh-CN"), Language::ChineseSimplified);
    assert_eq!(Language::from_tag("zh-TW"), Language::ChineseTraditional);
    assert_eq!(Language::from_tag("zh-Hant-HK"), Language::ChineseTraditional);
    assert_eq!(Language::from_tag("zh-Hans-HK"), Language::ChineseSimplified);
    assert_eq!(Language::from_tag("iw"), Language::Hebrew);
    assert_eq!(Language::from_tag("xx"), Language::English);
}

#[test]
fn every_language_reads_every_sample() {
    for lang in Language::ALL {
        for tex in SAMPLES {
            let s = say(tex, lang.tag());
            assert!(!s.is_empty(), "{}: {tex}", lang.tag());
            assert!(!s.contains('{'), "{}: unfilled slot in `{s}`", lang.tag());
        }
    }
}

#[test]
fn word_order_follows_the_language() {
    assert_eq!(say("x^2 + y^2 = z^2", "en"), "x squared plus y squared equals z squared");
    assert_eq!(say(r"\frac{a}{b}", "en"), "a over b");
    // Denominator first.
    let ja = say(r"\frac{a}{b}", "ja");
    assert!(ja.find('b').unwrap() < ja.find('a').unwrap(), "{ja}");
    let zh = say(r"\frac{a}{b}", "zh-CN");
    assert!(zh.find('b').unwrap() < zh.find('a').unwrap(), "{zh}");
}

#[test]
#[ignore]
fn dump_samples() {
    for lang in Language::ALL {
        println!("== {} ({})", lang.native_name(), lang.tag());
        for tex in SAMPLES {
            println!("{tex}\n    {}", say(tex, lang.tag()));
        }
    }
}
