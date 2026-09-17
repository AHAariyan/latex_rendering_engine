//! Regression corpus shared by the golden and subset tests.

/// (name, tex, display_mode)
pub const CORPUS: &[(&str, &str, bool)] = &[
    ("quadratic", r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}", true),
    ("basel", r"\sum_{n=1}^{\infty} \frac{1}{n^2} = \frac{\pi^2}{6}", true),
    ("gaussian", r"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}", true),
    ("euler", r"e^{i\pi} + 1 = 0", false),
    ("limit", r"\lim_{x \to 0} \frac{\sin x}{x} = 1", true),
    ("limit_inline", r"\lim_{x \to 0} \frac{\sin x}{x} = 1", false),
    ("binom", r"\binom{n}{k} = \frac{n!}{k!(n-k)!}", true),
    (
        "cases",
        r"f(x) = \begin{cases} x^2 & \text{if } x \ge 0 \\ -x & \text{otherwise} \end{cases}",
        true,
    ),
    ("pmatrix", r"A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}", true),
    (
        "bmatrix3",
        r"\begin{bmatrix} 1 & 0 & 0 \\ 0 & 1 & 0 \\ 0 & 0 & 1 \end{bmatrix}",
        true,
    ),
    ("aligned", r"\begin{aligned} a &= b + c \\ d + e &= f \end{aligned}", true),
    ("left_right", r"\left( \frac{a}{b} + \left[ \frac{c}{d} \right] \right)^2", true),
    ("nested_frac", r"\frac{1}{1 + \frac{1}{1 + \frac{1}{x}}}", true),
    ("sqrt_index", r"\sqrt[3]{x^3 + y^3} + \sqrt{\frac{a}{b}}", true),
    (
        "accents",
        r"\hat{x} \tilde{y} \bar{z} \vec{v} \dot{q} \ddot{q} \widehat{abc} \widetilde{xyz} \overline{AB} \underline{CD}",
        true,
    ),
    (
        "alphabets",
        r"\mathbf{Ab} \mathbb{RZ} \mathcal{FG} \mathfrak{gh} \mathsf{xy} \mathtt{01} \boldsymbol{\alpha\beta}",
        true,
    ),
    (
        "greek",
        r"\alpha\beta\gamma\delta\epsilon\varepsilon\zeta\eta\theta\vartheta\iota\kappa\lambda\mu\nu\xi\pi\rho\sigma\tau\upsilon\phi\varphi\chi\psi\omega \Gamma\Delta\Theta\Lambda\Xi\Pi\Sigma\Upsilon\Phi\Psi\Omega",
        true,
    ),
    (
        "relations",
        r"a \le b \ge c \ne d \approx e \equiv f \subset g \in h \to i \Rightarrow j \iff k",
        true,
    ),
    ("spacing", r"a+b-c\times d\cdot e = f, g; h \, i \; j \quad k \qquad l", true),
    (
        "scripts",
        r"x_i^2 + y^{a^{b^c}} + z_{i_{j_k}} + f'(x) + g''(x) + T_{\mathrm{max}}^{\mathrm{ref}}",
        true,
    ),
    (
        "big_ops_display",
        r"\sum_{i=1}^{n} \prod_{j=1}^{m} \int_0^1 \oint_C \bigcup_{k} \bigcap_{k} x",
        true,
    ),
    ("big_ops_inline", r"\sum_{i=1}^{n} \prod_{j=1}^{m} \int_0^1 \oint_C x", false),
    (
        "maxwell",
        r"\nabla \cdot \mathbf{E} = \frac{\rho}{\varepsilon_0}, \quad \nabla \times \mathbf{B} - \frac{1}{c^2}\frac{\partial \mathbf{E}}{\partial t} = \mu_0 \mathbf{J}",
        true,
    ),
    (
        "schrodinger",
        r"i\hbar \frac{\partial}{\partial t} \Psi(\mathbf{r},t) = \left[ -\frac{\hbar^2}{2m} \nabla^2 + V(\mathbf{r},t) \right] \Psi(\mathbf{r},t)",
        true,
    ),
    (
        "sized_delims",
        r"\bigl( \Bigl( \biggl( \Biggl( x \Biggr) \biggr) \Bigr) \bigr)",
        true,
    ),
    (
        "text_and_ops",
        r"\sin^2\theta + \cos^2\theta = 1, \quad \log_2 8 = 3, \quad \operatorname{argmax}_x f(x)",
        true,
    ),
    (
        "stack",
        r"\overset{\text{def}}{=} \quad \underset{x}{\max} \quad \stackrel{?}{=}",
        true,
    ),
    ("not_and_neg", r"a \not= b, \quad x \notin S, \quad \neg p \lor q", true),
    ("phantom", r"\frac{1}{\phantom{-}2} + \frac{1}{-2}", true),
    (
        "smallmatrix",
        r"\left( \begin{smallmatrix} a & b \\ c & d \end{smallmatrix} \right)",
        false,
    ),
    (
        "boxed_cancel",
        r"\boxed{E = mc^2} \quad \cancel{x} + \bcancel{y} + \xcancel{z}",
        true,
    ),
    ("braces", r"\underbrace{a + b + \cdots + z}_{26} \quad \overbrace{1 + 2}^{3}", true),
    (
        "xarrows",
        r"A \xrightarrow{f \circ g} B \xleftarrow[\text{under}]{} C \xLeftrightarrow{\sim} D",
        true,
    ),
    ("middle", r"\left\{ \frac{x}{2} \middle| x \in \mathbb{Z} \right\}", true),
    ("substack", r"\sum_{\substack{i < n \\ j < m}} a_{ij}", true),
    (
        "mod_choose",
        r"a \equiv b \pmod{n}, \quad a \bmod b, \quad {n \choose k}, \quad {a \over b + c}",
        true,
    ),
    ("genfrac", r"\genfrac(]{0pt}{0}{a}{b} \quad \genfrac{}{}{1pt}{}{x}{y}", true),
    (
        "array_rules",
        r"\begin{array}{|c|r|} \hline 1 & 22 \\[4pt] \hline 333 & 4 \\ \hline \end{array}",
        true,
    ),
    ("colors", r"\textcolor{red}{x} + \color{blue} y = \textcolor{#0a0}{z}", true),
    (
        "macros",
        r"\newcommand{\pd}[2]{\frac{\partial #1}{\partial #2}} \pd{f}{x} + \pd{f}{y}",
        true,
    ),
    (
        "mathop_class",
        r"\mathop{\mathrm{argmin}}\limits_{x} f(x), \quad a \mathrel{R} b, \quad a \mathbin{\ast} b",
        true,
    ),
    ("unicode", "α + β ≤ γ, ∑_{i} x_i, x → ∞", true),
];
