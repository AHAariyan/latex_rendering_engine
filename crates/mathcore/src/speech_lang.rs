//! Spoken mathematics in 35 languages.
//!
//! The speech writer composes English phrases ("squared", "is less than or
//! equal to") and templates whose numbered slots it fills with the spoken
//! parts ("{0} over {1}", "the square root of {0}"). This module turns each
//! into the chosen language. A template may move its slots wherever the
//! language's grammar wants them, so Japanese reads a fraction denominator
//! first ("{1}分の{0}") and Turkish puts the root after its radicand.
//!
//! Every phrase is listed once in `keys.rs` with a note for translators; each
//! language file translates all of them, which the tests enforce. A phrase
//! missing at run time falls back to English, so a gap degrades a reading
//! rather than breaking it.

mod ar;
mod bn;
mod de;
mod el;
mod es;
mod fa;
mod fr;
mod gu;
mod he;
mod hi;
mod id;
mod it;
mod ja;
#[cfg(test)]
mod keys;
mod kn;
mod ko;
mod ml;
mod mr;
mod ms;
mod nl;
mod pa;
mod pl;
mod pt;
mod ru;
mod sv;
mod sw;
mod ta;
mod te;
mod th;
mod tr;
mod uk;
mod ur;
mod vi;
mod zh_hans;
mod zh_hant;

#[cfg(test)]
use keys::KEYS;

/// A language for spoken mathematics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Language {
    #[default]
    /// English (English).
    English,
    /// Spanish (Español).
    Spanish,
    /// French (Français).
    French,
    /// German (Deutsch).
    German,
    /// Portuguese (Português).
    Portuguese,
    /// Italian (Italiano).
    Italian,
    /// Dutch (Nederlands).
    Dutch,
    /// Swedish (Svenska).
    Swedish,
    /// Polish (Polski).
    Polish,
    /// Russian (Русский).
    Russian,
    /// Ukrainian (Українська).
    Ukrainian,
    /// Greek (Ελληνικά).
    Greek,
    /// Turkish (Türkçe).
    Turkish,
    /// Arabic (العربية).
    Arabic,
    /// Hebrew (עברית).
    Hebrew,
    /// Persian (فارسی).
    Persian,
    /// Urdu (اردو).
    Urdu,
    /// Hindi (हिन्दी).
    Hindi,
    /// Bengali (বাংলা).
    Bengali,
    /// Marathi (मराठी).
    Marathi,
    /// Gujarati (ગુજરાતી).
    Gujarati,
    /// Punjabi (ਪੰਜਾਬੀ).
    Punjabi,
    /// Tamil (தமிழ்).
    Tamil,
    /// Telugu (తెలుగు).
    Telugu,
    /// Kannada (ಕನ್ನಡ).
    Kannada,
    /// Malayalam (മലയാളം).
    Malayalam,
    /// Chinese (Simplified) (简体中文).
    ChineseSimplified,
    /// Chinese (Traditional) (繁體中文).
    ChineseTraditional,
    /// Japanese (日本語).
    Japanese,
    /// Korean (한국어).
    Korean,
    /// Vietnamese (Tiếng Việt).
    Vietnamese,
    /// Thai (ไทย).
    Thai,
    /// Indonesian (Bahasa Indonesia).
    Indonesian,
    /// Malay (Bahasa Melayu).
    Malay,
    /// Swahili (Kiswahili).
    Swahili,
}

impl Language {
    pub const ALL: [Language; 35] = [
        Language::English,
        Language::Spanish,
        Language::French,
        Language::German,
        Language::Portuguese,
        Language::Italian,
        Language::Dutch,
        Language::Swedish,
        Language::Polish,
        Language::Russian,
        Language::Ukrainian,
        Language::Greek,
        Language::Turkish,
        Language::Arabic,
        Language::Hebrew,
        Language::Persian,
        Language::Urdu,
        Language::Hindi,
        Language::Bengali,
        Language::Marathi,
        Language::Gujarati,
        Language::Punjabi,
        Language::Tamil,
        Language::Telugu,
        Language::Kannada,
        Language::Malayalam,
        Language::ChineseSimplified,
        Language::ChineseTraditional,
        Language::Japanese,
        Language::Korean,
        Language::Vietnamese,
        Language::Thai,
        Language::Indonesian,
        Language::Malay,
        Language::Swahili,
    ];

    /// The language for a BCP 47 tag (`es`, `pt-BR`, `bn-BD`, `zh-TW`,
    /// `zh-Hans-CN`); English for a language not in the list.
    pub fn from_tag(tag: &str) -> Language {
        let lower = tag.to_ascii_lowercase().replace('_', "-");
        let mut parts = lower.split('-');
        let primary = parts.next().unwrap_or("");
        match primary {
            // Chinese: the script decides, else the region (Taiwan, Hong
            // Kong and Macau write Traditional characters).
            "zh" | "cmn" | "yue" => {
                let rest: Vec<&str> = parts.collect();
                let traditional = rest.contains(&"hant")
                    || (!rest.contains(&"hans") && rest.iter().any(|r| matches!(*r, "tw" | "hk" | "mo")))
                    || (primary == "yue" && !rest.contains(&"hans"));
                if traditional {
                    Language::ChineseTraditional
                } else {
                    Language::ChineseSimplified
                }
            }
            "en" => Language::English,
            "es" => Language::Spanish,
            "fr" => Language::French,
            "de" => Language::German,
            "pt" => Language::Portuguese,
            "it" => Language::Italian,
            "nl" => Language::Dutch,
            "sv" => Language::Swedish,
            "pl" => Language::Polish,
            "ru" => Language::Russian,
            "uk" => Language::Ukrainian,
            "el" => Language::Greek,
            "tr" => Language::Turkish,
            "ar" => Language::Arabic,
            "he" | "iw" => Language::Hebrew,
            "fa" => Language::Persian,
            "ur" => Language::Urdu,
            "hi" => Language::Hindi,
            "bn" => Language::Bengali,
            "mr" => Language::Marathi,
            "gu" => Language::Gujarati,
            "pa" => Language::Punjabi,
            "ta" => Language::Tamil,
            "te" => Language::Telugu,
            "kn" => Language::Kannada,
            "ml" => Language::Malayalam,
            "ja" => Language::Japanese,
            "ko" => Language::Korean,
            "vi" => Language::Vietnamese,
            "th" => Language::Thai,
            "id" | "in" => Language::Indonesian,
            "ms" => Language::Malay,
            "sw" => Language::Swahili,
            _ => Language::English,
        }
    }

    /// The BCP 47 tag, as `from_tag` reads it back.
    pub fn tag(self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Spanish => "es",
            Language::French => "fr",
            Language::German => "de",
            Language::Portuguese => "pt",
            Language::Italian => "it",
            Language::Dutch => "nl",
            Language::Swedish => "sv",
            Language::Polish => "pl",
            Language::Russian => "ru",
            Language::Ukrainian => "uk",
            Language::Greek => "el",
            Language::Turkish => "tr",
            Language::Arabic => "ar",
            Language::Hebrew => "he",
            Language::Persian => "fa",
            Language::Urdu => "ur",
            Language::Hindi => "hi",
            Language::Bengali => "bn",
            Language::Marathi => "mr",
            Language::Gujarati => "gu",
            Language::Punjabi => "pa",
            Language::Tamil => "ta",
            Language::Telugu => "te",
            Language::Kannada => "kn",
            Language::Malayalam => "ml",
            Language::ChineseSimplified => "zh-Hans",
            Language::ChineseTraditional => "zh-Hant",
            Language::Japanese => "ja",
            Language::Korean => "ko",
            Language::Vietnamese => "vi",
            Language::Thai => "th",
            Language::Indonesian => "id",
            Language::Malay => "ms",
            Language::Swahili => "sw",
        }
    }

    /// The language's name in itself, for a language picker.
    pub fn native_name(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Spanish => "Español",
            Language::French => "Français",
            Language::German => "Deutsch",
            Language::Portuguese => "Português",
            Language::Italian => "Italiano",
            Language::Dutch => "Nederlands",
            Language::Swedish => "Svenska",
            Language::Polish => "Polski",
            Language::Russian => "Русский",
            Language::Ukrainian => "Українська",
            Language::Greek => "Ελληνικά",
            Language::Turkish => "Türkçe",
            Language::Arabic => "العربية",
            Language::Hebrew => "עברית",
            Language::Persian => "فارسی",
            Language::Urdu => "اردو",
            Language::Hindi => "हिन्दी",
            Language::Bengali => "বাংলা",
            Language::Marathi => "मराठी",
            Language::Gujarati => "ગુજરાતી",
            Language::Punjabi => "ਪੰਜਾਬੀ",
            Language::Tamil => "தமிழ்",
            Language::Telugu => "తెలుగు",
            Language::Kannada => "ಕನ್ನಡ",
            Language::Malayalam => "മലയാളം",
            Language::ChineseSimplified => "简体中文",
            Language::ChineseTraditional => "繁體中文",
            Language::Japanese => "日本語",
            Language::Korean => "한국어",
            Language::Vietnamese => "Tiếng Việt",
            Language::Thai => "ไทย",
            Language::Indonesian => "Bahasa Indonesia",
            Language::Malay => "Bahasa Melayu",
            Language::Swahili => "Kiswahili",
        }
    }

    fn table(self) -> Option<&'static [(&'static str, &'static str)]> {
        Some(match self {
            Language::English => return None,
            Language::Spanish => es::PHRASES,
            Language::French => fr::PHRASES,
            Language::German => de::PHRASES,
            Language::Portuguese => pt::PHRASES,
            Language::Italian => it::PHRASES,
            Language::Dutch => nl::PHRASES,
            Language::Swedish => sv::PHRASES,
            Language::Polish => pl::PHRASES,
            Language::Russian => ru::PHRASES,
            Language::Ukrainian => uk::PHRASES,
            Language::Greek => el::PHRASES,
            Language::Turkish => tr::PHRASES,
            Language::Arabic => ar::PHRASES,
            Language::Hebrew => he::PHRASES,
            Language::Persian => fa::PHRASES,
            Language::Urdu => ur::PHRASES,
            Language::Hindi => hi::PHRASES,
            Language::Bengali => bn::PHRASES,
            Language::Marathi => mr::PHRASES,
            Language::Gujarati => gu::PHRASES,
            Language::Punjabi => pa::PHRASES,
            Language::Tamil => ta::PHRASES,
            Language::Telugu => te::PHRASES,
            Language::Kannada => kn::PHRASES,
            Language::Malayalam => ml::PHRASES,
            Language::ChineseSimplified => zh_hans::PHRASES,
            Language::ChineseTraditional => zh_hant::PHRASES,
            Language::Japanese => ja::PHRASES,
            Language::Korean => ko::PHRASES,
            Language::Vietnamese => vi::PHRASES,
            Language::Thai => th::PHRASES,
            Language::Indonesian => id::PHRASES,
            Language::Malay => ms::PHRASES,
            Language::Swahili => sw::PHRASES,
        })
    }
}

/// An English phrase in `lang`.
pub fn translate(lang: Language, english: &str) -> String {
    #[cfg(test)]
    tests::record(english);
    let Some(table) = lang.table() else { return english.to_string() };
    if let Some(t) = lookup(table, english) {
        return t.to_string();
    }
    // Superbrief drops a leading "the"; the table may only have the full phrase.
    if let Some(t) = lookup(table, &format!("the {english}")) {
        return t.to_string();
    }
    if let Some(rest) = english.strip_prefix("capital ") {
        let template = lookup(table, "capital {0}").unwrap_or("capital {0}");
        return template.replace("{0}", &translate(lang, rest));
    }
    english.to_string()
}

/// A template in `lang` with its slots filled: `fill(Japanese, "{0} over {1}", ["a", "b"])`.
pub fn fill(lang: Language, template: &str, args: &[&str]) -> String {
    let mut t = translate(lang, template);
    for (i, a) in args.iter().enumerate() {
        t = t.replace(&format!("{{{i}}}"), a);
    }
    t
}

fn lookup(table: &[(&str, &'static str)], english: &str) -> Option<&'static str> {
    table.iter().find(|(e, _)| *e == english).map(|(_, t)| *t).filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests;
