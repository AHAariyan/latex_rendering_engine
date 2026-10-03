//! Spoken mathematics in languages other than English.
//!
//! The speech writer composes English phrases ("the fraction", "squared",
//! "is less than or equal to"); this module turns each into the chosen
//! language. A phrase without a translation falls back to its English, so a
//! missing entry degrades a reading rather than breaking it. Phrases with
//! parts in them are templates whose `{}` the writer fills, which lets each
//! language put the parts where its grammar wants them.

/// A language for spoken mathematics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Language {
    #[default]
    English,
    Spanish,
    French,
    German,
    Portuguese,
    Bengali,
    Hindi,
}

impl Language {
    pub const ALL: [Language; 7] = [
        Language::English,
        Language::Spanish,
        Language::French,
        Language::German,
        Language::Portuguese,
        Language::Bengali,
        Language::Hindi,
    ];

    /// The language for a BCP 47 tag or its primary subtag (`es`, `pt-BR`,
    /// `bn-BD`); English for anything else.
    pub fn from_tag(tag: &str) -> Language {
        let primary = tag.split(['-', '_']).next().unwrap_or("").to_ascii_lowercase();
        match primary.as_str() {
            "es" => Language::Spanish,
            "fr" => Language::French,
            "de" => Language::German,
            "pt" => Language::Portuguese,
            "bn" => Language::Bengali,
            "hi" => Language::Hindi,
            _ => Language::English,
        }
    }

    pub fn tag(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::German => "de",
            Language::Portuguese => "pt",
            Language::Bengali => "bn",
            Language::Hindi => "hi",
        }
    }

    fn column(self) -> Option<usize> {
        Some(match self {
            Language::English => return None,
            Language::Spanish => 0,
            Language::French => 1,
            Language::German => 2,
            Language::Portuguese => 3,
            Language::Bengali => 4,
            Language::Hindi => 5,
        })
    }
}

/// An English phrase in `lang`.
pub fn translate(lang: Language, english: &str) -> String {
    let Some(col) = lang.column() else { return english.to_string() };
    if let Some(t) = lookup(col, english) {
        return t.to_string();
    }
    // Superbrief drops a leading "the"; the table may only have the full phrase.
    if let Some(t) = lookup(col, &format!("the {english}")) {
        return t.to_string();
    }
    if let Some(rest) = english.strip_prefix("capital ") {
        let template = lookup(col, "capital {}").unwrap_or("capital {}");
        return template.replace("{}", &translate(lang, rest));
    }
    english.to_string()
}

fn lookup(col: usize, english: &str) -> Option<&'static str> {
    PHRASES
        .iter()
        .find(|(e, _)| *e == english)
        .map(|(_, t)| t[col])
        .filter(|t| !t.is_empty())
}

/// English, then Spanish, French, German, Portuguese, Bengali, Hindi.
/// An empty translation falls back to English.
#[rustfmt::skip]
static PHRASES: &[(&str, [&str; 6])] = &[
    // Structure
    ("squared", ["al cuadrado", "au carré", "Quadrat", "ao quadrado", "এর বর্গ", "का वर्ग"]),
    ("cubed", ["al cubo", "au cube", "hoch drei", "ao cubo", "এর ঘন", "का घन"]),
    ("to the", ["elevado a", "puissance", "hoch", "elevado a", "এর ঘাত", "की घात"]),
    ("to the power of", ["elevado a la potencia", "à la puissance", "hoch", "elevado à potência", "এর ঘাত", "की घात"]),
    ("end power", ["fin de la potencia", "fin de la puissance", "Ende der Potenz", "fim da potência", "ঘাত শেষ", "घात समाप्त"]),
    ("sub", ["subíndice", "indice", "Index", "índice", "নিম্নে", "पादांक"]),
    ("end sub", ["fin del subíndice", "fin de l'indice", "Ende des Index", "fim do índice", "নিম্নলিপি শেষ", "पादांक समाप्त"]),
    ("the fraction", ["la fracción", "la fraction", "der Bruch", "a fração", "ভগ্নাংশ", "भिन्न"]),
    ("over", ["sobre", "sur", "durch", "sobre", "বাই", "बटा"]),
    ("end fraction", ["fin de la fracción", "fin de la fraction", "Ende des Bruchs", "fim da fração", "ভগ্নাংশ শেষ", "भिन्न समाप्त"]),
    ("choose", ["sobre", "parmi", "über", "escolhe", "চয়ন", "चयन"]),
    ("the square root of", ["la raíz cuadrada de", "la racine carrée de", "die Wurzel aus", "a raiz quadrada de", "বর্গমূল", "वर्गमूल"]),
    ("square root of", ["raíz cuadrada de", "racine carrée de", "Wurzel aus", "raiz quadrada de", "বর্গমূল", "वर्गमूल"]),
    ("the cube root of", ["la raíz cúbica de", "la racine cubique de", "die dritte Wurzel aus", "a raiz cúbica de", "ঘনমূল", "घनमूल"]),
    ("the {}th root of", ["la raíz {}-ésima de", "la racine {}-ième de", "die {}-te Wurzel aus", "a raiz {}-ésima de", "{}-তম মূল", "{}वाँ मूल"]),
    ("{}th root of", ["raíz {}-ésima de", "racine {}-ième de", "{}-te Wurzel aus", "raiz {}-ésima de", "{}-তম মূল", "{}वाँ मूल"]),
    ("end root", ["fin de la raíz", "fin de la racine", "Ende der Wurzel", "fim da raiz", "মূল শেষ", "मूल समाप्त"]),
    ("from", ["desde", "de", "von", "de", "নিম্নসীমা", "निम्न सीमा"]),
    ("to", ["hasta", "à", "bis", "até", "ঊর্ধ্বসীমা", "उच्च सीमा"]),
    ("of", ["de", "de", "von", "de", "এর", "का"]),
    ("with", ["con", "avec", "mit", "com", "সহ", "के साथ"]),
    ("above,", ["arriba,", "au-dessus,", "oben,", "acima,", "উপরে,", "ऊपर,"]),
    ("below,", ["abajo,", "au-dessous,", "unten,", "abaixo,", "নিচে,", "नीचे,"]),
    ("under", ["bajo", "sous", "unter", "sob", "নিচে", "नीचे"]),
    ("bar", ["barra", "barre", "quer", "barra", "বার", "बार"]),
    ("underlined", ["subrayado", "souligné", "unterstrichen", "sublinhado", "নিম্নরেখা", "रेखांकित"]),
    ("boxed,", ["en recuadro,", "encadré,", "eingerahmt,", "em caixa,", "বাক্সে,", "बॉक्स में,"]),
    (", end box", [", fin del recuadro", ", fin du cadre", ", Ende des Rahmens", ", fim da caixa", ", বাক্স শেষ", ", बॉक्स समाप्त"]),
    ("crossed out,", ["tachado,", "barré,", "durchgestrichen,", "riscado,", "কাটা,", "काटा हुआ,"]),
    (", end crossed out", [", fin del tachado", ", fin du barré", ", Ende der Streichung", ", fim do riscado", ", কাটা শেষ", ", काटा हुआ समाप्त"]),
    (", end sub", [", fin del subíndice", ", fin de l'indice", ", Ende des Index", ", fim do índice", ", নিম্নলিপি শেষ", ", पादांक समाप्त"]),
    ("over brace of", ["llave superior de", "accolade supérieure de", "obere Klammer von", "chave superior de", "উপরের বন্ধনী", "ऊपरी कोष्ठक"]),
    ("under brace of", ["llave inferior de", "accolade inférieure de", "untere Klammer von", "chave inferior de", "নিচের বন্ধনী", "निचला कोष्ठक"]),
    ("blank", ["vacío", "vide", "leer", "vazio", "ফাঁকা", "रिक्त"]),
    (", equation {}", [", ecuación {}", ", équation {}", ", Gleichung {}", ", equação {}", ", সমীকরণ {}", ", समीकरण {}"]),
    ("the {} by {} matrix,", ["la matriz de {} por {},", "la matrice {} par {},", "die {}-mal-{}-Matrix,", "a matriz {} por {},", "{} বাই {} ম্যাট্রিক্স,", "{} गुणा {} आव्यूह,"]),
    ("{} by {} matrix,", ["matriz de {} por {},", "matrice {} par {},", "{}-mal-{}-Matrix,", "matriz {} por {},", "{} বাই {} ম্যাট্রিক্স,", "{} गुणा {} आव्यूह,"]),
    ("{} rows,", ["{} filas,", "{} lignes,", "{} Zeilen,", "{} linhas,", "{} সারি,", "{} पंक्तियाँ,"]),
    ("row {},", ["fila {},", "ligne {},", "Zeile {},", "linha {},", "সারি {},", "पंक्ति {},"]),
    ("end matrix", ["fin de la matriz", "fin de la matrice", "Ende der Matrix", "fim da matriz", "ম্যাট্রিক্স শেষ", "आव्यूह समाप्त"]),
    ("end rows", ["fin de las filas", "fin des lignes", "Ende der Zeilen", "fim das linhas", "সারি শেষ", "पंक्तियाँ समाप्त"]),
    (", end {}", [", fin de {}", ", fin de {}", ", Ende {}", ", fim de {}", ", {} শেষ", ", {} समाप्त"]),
    ("capital {}", ["{} mayúscula", "{} majuscule", "großes {}", "{} maiúsculo", "বড় হাতের {}", "बड़ा {}"]),
    // Grouping
    ("open paren", ["abre paréntesis", "parenthèse ouvrante", "Klammer auf", "abre parênteses", "প্রথম বন্ধনী শুরু", "कोष्ठक खुला"]),
    ("close paren", ["cierra paréntesis", "parenthèse fermante", "Klammer zu", "fecha parênteses", "প্রথম বন্ধনী শেষ", "कोष्ठक बंद"]),
    ("open bracket", ["abre corchete", "crochet ouvrant", "eckige Klammer auf", "abre colchete", "তৃতীয় বন্ধনী শুরু", "वर्ग कोष्ठक खुला"]),
    ("close bracket", ["cierra corchete", "crochet fermant", "eckige Klammer zu", "fecha colchete", "তৃতীয় বন্ধনী শেষ", "वर्ग कोष्ठक बंद"]),
    ("open brace", ["abre llave", "accolade ouvrante", "geschweifte Klammer auf", "abre chave", "দ্বিতীয় বন্ধনী শুরু", "धनु कोष्ठक खुला"]),
    ("close brace", ["cierra llave", "accolade fermante", "geschweifte Klammer zu", "fecha chave", "দ্বিতীয় বন্ধনী শেষ", "धनु कोष्ठक बंद"]),
    ("open angle bracket", ["abre ángulo", "chevron ouvrant", "spitze Klammer auf", "abre ângulo", "কোণ বন্ধনী শুরু", "कोण कोष्ठक खुला"]),
    ("close angle bracket", ["cierra ángulo", "chevron fermant", "spitze Klammer zu", "fecha ângulo", "কোণ বন্ধনী শেষ", "कोण कोष्ठक बंद"]),
    ("the absolute value of", ["el valor absoluto de", "la valeur absolue de", "der Betrag von", "o valor absoluto de", "পরম মান", "निरपेक्ष मान"]),
    ("the norm of", ["la norma de", "la norme de", "die Norm von", "a norma de", "নর্ম", "मानक"]),
    ("the floor of", ["el piso de", "la partie entière de", "die Abrundung von", "o piso de", "ফ্লোর", "फ़्लोर"]),
    ("the ceiling of", ["el techo de", "la partie entière supérieure de", "die Aufrundung von", "o teto de", "সিলিং", "सीलिंग"]),
    ("absolute value", ["valor absoluto", "valeur absolue", "Betrag", "valor absoluto", "পরম মান", "निरपेक्ष मान"]),
    ("norm", ["norma", "norme", "Norm", "norma", "নর্ম", "मानक"]),
    ("floor", ["piso", "partie entière", "Abrundung", "piso", "ফ্লোর", "फ़्लोर"]),
    ("ceiling", ["techo", "partie entière supérieure", "Aufrundung", "teto", "সিলিং", "सीलिंग"]),
    ("group", ["grupo", "groupe", "Gruppe", "grupo", "গুচ্ছ", "समूह"]),
    // Accents
    ("hat", ["sombrero", "chapeau", "Dach", "chapéu", "হ্যাট", "हैट"]),
    ("tilde", ["tilde", "tilde", "Tilde", "til", "টিল্ডা", "टिल्डा"]),
    ("vector", ["vector", "vecteur", "Vektor", "vetor", "ভেক্টর", "सदिश"]),
    ("dot", ["punto", "point", "Punkt", "ponto", "ডট", "बिंदु"]),
    ("double dot", ["doble punto", "deux points", "zwei Punkte", "dois pontos", "ডাবল ডট", "दोहरा बिंदु"]),
    // Large operators
    ("the sum", ["la suma", "la somme", "die Summe", "a soma", "যোগফল", "योग"]),
    ("the product", ["el producto", "le produit", "das Produkt", "o produto", "গুণফল", "गुणनफल"]),
    ("the integral", ["la integral", "l'intégrale", "das Integral", "a integral", "সমাকলন", "समाकल"]),
    ("the double integral", ["la integral doble", "l'intégrale double", "das Doppelintegral", "a integral dupla", "দ্বি-সমাকলন", "द्वि-समाकल"]),
    ("the triple integral", ["la integral triple", "l'intégrale triple", "das Dreifachintegral", "a integral tripla", "ত্রি-সমাকলন", "त्रि-समाकल"]),
    ("the contour integral", ["la integral de contorno", "l'intégrale curviligne", "das Kurvenintegral", "a integral de contorno", "কনট্যুর সমাকলন", "कंटूर समाकल"]),
    ("the union", ["la unión", "l'union", "die Vereinigung", "a união", "সংযোগ", "सम्मिलन"]),
    ("the intersection", ["la intersección", "l'intersection", "der Durchschnitt", "a interseção", "ছেদ", "सर्वनिष्ठ"]),
    ("the coproduct", ["el coproducto", "le coproduit", "das Koprodukt", "o coproduto", "সহগুণফল", "सहगुणनफल"]),
    // Operators and relations
    ("plus", ["más", "plus", "plus", "mais", "যোগ", "धन"]),
    ("minus", ["menos", "moins", "minus", "menos", "বিয়োগ", "ऋण"]),
    ("plus or minus", ["más o menos", "plus ou moins", "plus minus", "mais ou menos", "যোগ বা বিয়োগ", "धन या ऋण"]),
    ("minus or plus", ["menos o más", "moins ou plus", "minus plus", "menos ou mais", "বিয়োগ বা যোগ", "ऋण या धन"]),
    ("times", ["por", "fois", "mal", "vezes", "গুণ", "गुणा"]),
    ("divided by", ["dividido entre", "divisé par", "geteilt durch", "dividido por", "ভাগ", "भाग"]),
    ("star", ["asterisco", "étoile", "Stern", "asterisco", "তারকা", "तारा"]),
    ("equals", ["igual a", "égale", "gleich", "igual a", "সমান", "बराबर"]),
    ("is not equal to", ["no es igual a", "n'est pas égal à", "ungleich", "não é igual a", "সমান নয়", "बराबर नहीं"]),
    ("is less than", ["es menor que", "est inférieur à", "kleiner als", "é menor que", "ছোট", "से कम"]),
    ("is greater than", ["es mayor que", "est supérieur à", "größer als", "é maior que", "বড়", "से अधिक"]),
    ("is less than or equal to", ["es menor o igual que", "est inférieur ou égal à", "kleiner oder gleich", "é menor ou igual a", "ছোট বা সমান", "से कम या बराबर"]),
    ("is greater than or equal to", ["es mayor o igual que", "est supérieur ou égal à", "größer oder gleich", "é maior ou igual a", "বড় বা সমান", "से अधिक या बराबर"]),
    ("is approximately", ["es aproximadamente", "est environ égal à", "ungefähr gleich", "é aproximadamente", "প্রায় সমান", "लगभग बराबर"]),
    ("is equivalent to", ["es equivalente a", "est équivalent à", "äquivalent zu", "é equivalente a", "সমতুল্য", "तुल्य"]),
    ("is similar to", ["es similar a", "est semblable à", "ähnlich zu", "é semelhante a", "সদৃশ", "समरूप"]),
    ("is congruent to", ["es congruente con", "est congru à", "kongruent zu", "é congruente a", "সর্বসম", "सर्वांगसम"]),
    ("is proportional to", ["es proporcional a", "est proportionnel à", "proportional zu", "é proporcional a", "সমানুপাতিক", "समानुपाती"]),
    ("is in", ["pertenece a", "appartient à", "Element von", "pertence a", "এর উপাদান", "का सदस्य"]),
    ("is not in", ["no pertenece a", "n'appartient pas à", "nicht Element von", "não pertence a", "এর উপাদান নয়", "का सदस्य नहीं"]),
    ("contains", ["contiene", "contient", "enthält", "contém", "ধারণ করে", "में है"]),
    ("is a proper subset of", ["es subconjunto propio de", "est un sous-ensemble strict de", "echte Teilmenge von", "é subconjunto próprio de", "প্রকৃত উপসেট", "उचित उपसमुच्चय"]),
    ("is a subset of", ["es subconjunto de", "est un sous-ensemble de", "Teilmenge von", "é subconjunto de", "উপসেট", "उपसमुच्चय"]),
    ("is a proper superset of", ["es superconjunto propio de", "est un sur-ensemble strict de", "echte Obermenge von", "é superconjunto próprio de", "প্রকৃত অধিসেট", "उचित अधिसमुच्चय"]),
    ("is a superset of", ["es superconjunto de", "est un sur-ensemble de", "Obermenge von", "é superconjunto de", "অধিসেট", "अधिसमुच्चय"]),
    ("union", ["unión", "union", "vereinigt", "união", "সংযোগ", "सम्मिलन"]),
    ("intersection", ["intersección", "inter", "geschnitten", "interseção", "ছেদ", "सर्वनिष्ठ"]),
    ("set minus", ["menos", "privé de", "ohne", "menos", "বাদে", "को छोड़कर"]),
    ("the empty set", ["el conjunto vacío", "l'ensemble vide", "die leere Menge", "o conjunto vazio", "ফাঁকা সেট", "रिक्त समुच्चय"]),
    ("infinity", ["infinito", "l'infini", "unendlich", "infinito", "অসীম", "अनंत"]),
    ("partial", ["parcial", "d rond", "partiell", "parcial", "আংশিক", "आंशिक"]),
    ("del", ["nabla", "nabla", "Nabla", "nabla", "ডেল", "डेल"]),
    ("goes to", ["tiende a", "tend vers", "gegen", "tende a", "অভিমুখে", "की ओर"]),
    ("comes from", ["viene de", "vient de", "kommt von", "vem de", "থেকে আসে", "से आता है"]),
    ("if and only if", ["si y solo si", "si et seulement si", "genau dann wenn", "se e somente se", "যদি এবং কেবল যদি", "यदि और केवल यदि"]),
    ("implies", ["implica", "implique", "impliziert", "implica", "নির্দেশ করে", "का तात्पर्य"]),
    ("is implied by", ["es implicado por", "est impliqué par", "folgt aus", "é implicado por", "থেকে আসে", "से निकलता है"]),
    ("maps to", ["se aplica en", "a pour image", "wird abgebildet auf", "é levado em", "এ যায়", "पर जाता है"]),
    ("for all", ["para todo", "pour tout", "für alle", "para todo", "সকল", "सभी के लिए"]),
    ("there exists", ["existe", "il existe", "es gibt", "existe", "বিদ্যমান", "अस्तित्व है"]),
    ("there does not exist", ["no existe", "il n'existe pas", "es gibt kein", "não existe", "বিদ্যমান নয়", "अस्तित्व नहीं"]),
    ("not", ["no", "non", "nicht", "não", "নয়", "नहीं"]),
    ("and", ["y", "et", "und", "e", "এবং", "और"]),
    ("or", ["o", "ou", "oder", "ou", "অথবা", "या"]),
    ("direct sum", ["suma directa", "somme directe", "direkte Summe", "soma direta", "প্রত্যক্ষ যোগ", "प्रत्यक्ष योग"]),
    ("tensor", ["tensor", "tenseur", "Tensor", "tensor", "টেনসর", "टेंसर"]),
    ("composed with", ["compuesto con", "rond", "verkettet mit", "composto com", "সংযোজিত", "संयुक्त"]),
    ("double bar", ["doble barra", "double barre", "Doppelstrich", "barra dupla", "দ্বৈত বার", "दोहरी बार"]),
    ("colon", ["dos puntos", "deux-points", "Doppelpunkt", "dois-pontos", "কোলন", "कोलन"]),
    ("factorial", ["factorial", "factorielle", "Fakultät", "fatorial", "ফ্যাক্টোরিয়াল", "क्रमगुणित"]),
    ("prime", ["prima", "prime", "Strich", "linha", "প্রাইম", "प्राइम"]),
    ("double prime", ["doble prima", "seconde", "zwei Strich", "duas linhas", "ডাবল প্রাইম", "डबल प्राइम"]),
    ("triple prime", ["triple prima", "tierce", "drei Strich", "três linhas", "ট্রিপল প্রাইম", "ट्रिपल प्राइम"]),
    ("and so on", ["etcétera", "et ainsi de suite", "und so weiter", "e assim por diante", "ইত্যাদি", "इत्यादि"]),
    ("degrees", ["grados", "degrés", "Grad", "graus", "ডিগ্রি", "डिग्री"]),
    ("percent", ["por ciento", "pour cent", "Prozent", "por cento", "শতাংশ", "प्रतिशत"]),
    ("h bar", ["h barra", "h barre", "h quer", "h cortado", "এইচ বার", "एच बार"]),
    // Greek, where the name changes
    ("alpha", ["alfa", "alpha", "Alpha", "alfa", "আলফা", "अल्फा"]),
    ("beta", ["beta", "bêta", "Beta", "beta", "বিটা", "बीटा"]),
    ("gamma", ["gamma", "gamma", "Gamma", "gama", "গামা", "गामा"]),
    ("delta", ["delta", "delta", "Delta", "delta", "ডেল্টা", "डेल्टा"]),
    ("epsilon", ["épsilon", "epsilon", "Epsilon", "épsilon", "এপসাইলন", "एप्सिलॉन"]),
    ("theta", ["theta", "thêta", "Theta", "teta", "থিটা", "थीटा"]),
    ("lambda", ["lambda", "lambda", "Lambda", "lambda", "ল্যামডা", "लैम्डा"]),
    ("mu", ["mu", "mu", "My", "mi", "মিউ", "म्यू"]),
    ("pi", ["pi", "pi", "Pi", "pi", "পাই", "पाई"]),
    ("rho", ["ro", "rhô", "Rho", "rô", "রো", "रो"]),
    ("sigma", ["sigma", "sigma", "Sigma", "sigma", "সিগমা", "सिग्मा"]),
    ("tau", ["tau", "tau", "Tau", "tau", "টাউ", "टाउ"]),
    ("phi", ["fi", "phi", "Phi", "fi", "ফাই", "फाई"]),
    ("chi", ["ji", "khi", "Chi", "qui", "কাই", "काई"]),
    ("psi", ["psi", "psi", "Psi", "psi", "সাই", "साई"]),
    ("omega", ["omega", "oméga", "Omega", "ômega", "ওমেগা", "ओमेगा"]),
    ("xi", ["xi", "ksi", "Xi", "csi", "জাই", "जाई"]),
    ("eta", ["eta", "êta", "Eta", "eta", "ইটা", "ईटा"]),
    ("zeta", ["zeta", "dzêta", "Zeta", "zeta", "জিটা", "ज़ीटा"]),
    ("kappa", ["kappa", "kappa", "Kappa", "capa", "কাপা", "कप्पा"]),
    ("nu", ["nu", "nu", "Ny", "ni", "নিউ", "न्यू"]),
    ("iota", ["iota", "iota", "Iota", "iota", "আইওটা", "आयोटा"]),
    ("upsilon", ["ípsilon", "upsilon", "Ypsilon", "úpsilon", "আপসাইলন", "अप्सिलॉन"]),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a11y::{speech_with, SpeechOptions, Verbosity};
    use crate::parse;

    fn say(tex: &str, lang: &str) -> String {
        let opts = SpeechOptions {
            language: Language::from_tag(lang),
            verbosity: Verbosity::Brief,
        };
        speech_with(&parse(tex).unwrap(), &opts)
    }

    #[test]
    fn languages_read_a_formula() {
        assert_eq!(say("x^2 + y^2 = z^2", "en"), "x squared plus y squared equals z squared");
        assert_eq!(
            say("x^2 + y^2 = z^2", "es-MX"),
            "x al cuadrado más y al cuadrado igual a z al cuadrado"
        );
        assert_eq!(say("x^2 + y^2 = z^2", "fr"), "x au carré plus y au carré égale z au carré");
        assert_eq!(say("x^2 + y^2 = z^2", "de"), "x Quadrat plus y Quadrat gleich z Quadrat");
        assert_eq!(
            say("x^2 + y^2 = z^2", "pt-BR"),
            "x ao quadrado mais y ao quadrado igual a z ao quadrado"
        );
        assert_eq!(say("x^2 + y^2 = z^2", "bn-BD"), "x এর বর্গ যোগ y এর বর্গ সমান z এর বর্গ");
        assert_eq!(say(r"\frac{a+b}{c}", "es"), "la fracción a más b sobre c,");
        assert_eq!(say(r"\sqrt[3]{x}", "fr"), "la racine cubique de x");
        assert_eq!(say(r"\sqrt[4]{x}", "de"), "die 4-te Wurzel aus x");
        assert_eq!(say(r"\Gamma", "es"), "gamma mayúscula");
        assert_eq!(
            say(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}", "es"),
            "abre paréntesis la matriz de 2 por 2, fila 1, a, b, fila 2, c, d, cierra paréntesis"
        );
        assert_eq!(say(r"x \le 1 \tag{2}", "hi"), "x से कम या बराबर 1, समीकरण 2");
    }

    #[test]
    fn unknown_tags_and_missing_phrases_fall_back_to_english() {
        assert_eq!(Language::from_tag("ja"), Language::English);
        assert_eq!(translate(Language::Spanish, "a phrase nobody wrote"), "a phrase nobody wrote");
        for lang in Language::ALL {
            assert_eq!(Language::from_tag(lang.tag()), lang);
            // Every formula reads in every language.
            assert!(!say(r"\int_0^1 \frac{\sqrt{x}}{1+x^2}\,dx", lang.tag()).is_empty());
        }
    }

    #[test]
    fn every_template_keeps_its_slots() {
        for (english, row) in PHRASES {
            let slots = english.matches("{}").count();
            for t in row.iter().filter(|t| !t.is_empty()) {
                assert_eq!(t.matches("{}").count(), slots, "{english} -> {t}");
            }
        }
    }
}
