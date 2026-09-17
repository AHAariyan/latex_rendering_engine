//! `cargo bench -p mathcore` — parse + layout throughput for typical formulas.
use criterion::{criterion_group, criterion_main, Criterion};
use mathcore::{MathFont, RenderOptions};

const FONT: &[u8] = include_bytes!("../../../assets/fonts/latinmodern-math.otf");

fn bench(c: &mut Criterion) {
    let font = MathFont::from_bytes(FONT).unwrap();
    let opts = RenderOptions::default();
    let cases = [
        ("inline_x2", "x^2 + y^2 = z^2"),
        ("quadratic", r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"),
        ("sum_frac", r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}"),
        ("matrix", r"\begin{pmatrix} a & b & c \\ d & e & f \\ g & h & i \end{pmatrix}"),
        (
            "schrodinger",
            r"i\hbar \frac{\partial}{\partial t} \Psi(\mathbf{r},t) = \left[ -\frac{\hbar^2}{2m} \nabla^2 + V(\mathbf{r},t) \right] \Psi(\mathbf{r},t)",
        ),
    ];
    for (name, tex) in cases {
        c.bench_function(name, |b| {
            b.iter(|| mathcore::render(&font, std::hint::black_box(tex), &opts).unwrap())
        });
    }
    c.bench_function("font_load", |b| {
        b.iter(|| MathFont::from_bytes(std::hint::black_box(FONT)).unwrap())
    });
}

criterion_group!(benches, bench);
criterion_main!(benches);
