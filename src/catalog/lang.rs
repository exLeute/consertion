//! Languages for the translator. Each is a "unit" of the online Translate category, so search,
//! favorites and recents work like any other conversion. Text goes out only when translating.

use super::{Builder, Kind};

/// (code, English name, native name). Codes are ISO 639-1 (plus region where it matters);
/// `crate::translate` maps them to each provider's spelling. Index 0 is source-only.
pub const LANGS: &[(&str, &str, &str)] = &[
    ("auto", "Detect language", "auto detect any"),
    ("en", "English", "english"),
    ("uk", "Ukrainian", "українська"),
    ("de", "German", "deutsch"),
    ("fr", "French", "français"),
    ("es", "Spanish", "español"),
    ("it", "Italian", "italiano"),
    ("pt", "Portuguese", "português"),
    ("pt-BR", "Portuguese (Brazil)", "português brasileiro"),
    ("pl", "Polish", "polski"),
    ("nl", "Dutch", "nederlands"),
    ("ru", "Russian", "русский"),
    ("zh-CN", "Chinese (Simplified)", "中文 简体 mandarin"),
    ("zh-TW", "Chinese (Traditional)", "中文 繁體"),
    ("ja", "Japanese", "日本語"),
    ("ko", "Korean", "한국어"),
    ("ar", "Arabic", "العربية"),
    ("tr", "Turkish", "türkçe"),
    ("cs", "Czech", "čeština"),
    ("sk", "Slovak", "slovenčina"),
    ("hu", "Hungarian", "magyar"),
    ("ro", "Romanian", "română"),
    ("bg", "Bulgarian", "български"),
    ("el", "Greek", "ελληνικά"),
    ("sv", "Swedish", "svenska"),
    ("da", "Danish", "dansk"),
    ("no", "Norwegian", "norsk bokmål"),
    ("fi", "Finnish", "suomi"),
    ("et", "Estonian", "eesti"),
    ("lv", "Latvian", "latviešu"),
    ("lt", "Lithuanian", "lietuvių"),
    ("sl", "Slovenian", "slovenščina"),
    ("hr", "Croatian", "hrvatski"),
    ("sr", "Serbian", "српски srpski"),
    ("bs", "Bosnian", "bosanski"),
    ("mk", "Macedonian", "македонски"),
    ("sq", "Albanian", "shqip"),
    ("be", "Belarusian", "беларуская"),
    ("ka", "Georgian", "ქართული"),
    ("hy", "Armenian", "հայերեն"),
    ("az", "Azerbaijani", "azərbaycan"),
    ("kk", "Kazakh", "қазақ"),
    ("uz", "Uzbek", "oʻzbek"),
    ("ky", "Kyrgyz", "кыргызча"),
    ("tg", "Tajik", "тоҷикӣ"),
    ("tk", "Turkmen", "türkmen"),
    ("mn", "Mongolian", "монгол"),
    ("he", "Hebrew", "עברית"),
    ("fa", "Persian", "فارسی farsi"),
    ("ur", "Urdu", "اردو"),
    ("hi", "Hindi", "हिन्दी"),
    ("bn", "Bengali", "বাংলা bangla"),
    ("pa", "Punjabi", "ਪੰਜਾਬੀ"),
    ("gu", "Gujarati", "ગુજરાતી"),
    ("mr", "Marathi", "मराठी"),
    ("ne", "Nepali", "नेपाली"),
    ("si", "Sinhala", "සිංහල"),
    ("ta", "Tamil", "தமிழ்"),
    ("te", "Telugu", "తెలుగు"),
    ("kn", "Kannada", "ಕನ್ನಡ"),
    ("ml", "Malayalam", "മലയാളം"),
    ("th", "Thai", "ไทย"),
    ("vi", "Vietnamese", "tiếng việt"),
    ("id", "Indonesian", "bahasa indonesia"),
    ("ms", "Malay", "bahasa melayu"),
    ("tl", "Filipino", "tagalog"),
    ("km", "Khmer", "ខ្មែរ cambodian"),
    ("lo", "Lao", "ລາວ"),
    ("my", "Burmese", "မြန်မာ myanmar"),
    ("jv", "Javanese", "basa jawa"),
    ("su", "Sundanese", "basa sunda"),
    ("ceb", "Cebuano", "sinugbuanong"),
    ("sw", "Swahili", "kiswahili"),
    ("am", "Amharic", "አማርኛ"),
    ("so", "Somali", "soomaali"),
    ("ha", "Hausa", "hausa"),
    ("yo", "Yoruba", "yorùbá"),
    ("ig", "Igbo", "igbo"),
    ("zu", "Zulu", "isizulu"),
    ("xh", "Xhosa", "isixhosa"),
    ("af", "Afrikaans", "afrikaans"),
    ("st", "Sesotho", "sesotho"),
    ("sn", "Shona", "chishona"),
    ("mg", "Malagasy", "malagasy"),
    ("ps", "Pashto", "پښتو"),
    ("sd", "Sindhi", "سنڌي"),
    ("ku", "Kurdish", "kurdî"),
    ("ug", "Uyghur", "ئۇيغۇرچە"),
    ("tt", "Tatar", "татар"),
    ("ca", "Catalan", "català"),
    ("eu", "Basque", "euskara"),
    ("gl", "Galician", "galego"),
    ("cy", "Welsh", "cymraeg"),
    ("ga", "Irish", "gaeilge"),
    ("gd", "Scottish Gaelic", "gàidhlig"),
    ("is", "Icelandic", "íslenska"),
    ("lb", "Luxembourgish", "lëtzebuergesch"),
    ("mt", "Maltese", "malti"),
    ("co", "Corsican", "corsu"),
    ("eo", "Esperanto", "esperanto"),
    ("la", "Latin", "latina"),
    ("yi", "Yiddish", "ייִדיש"),
    ("ht", "Haitian Creole", "kreyòl ayisyen"),
    ("haw", "Hawaiian", "ʻōlelo hawaiʻi"),
    ("mi", "Maori", "te reo māori"),
    ("sm", "Samoan", "gagana samoa"),
];

pub fn register(b: &mut Builder) {
    b.category("translate", "Translate (online)", true);
    for (i, &(code, name, native)) in LANGS.iter().enumerate() {
        let common = i < 30;
        b.add(name, code, Kind::Lang(i), &format!("language translate translation translator {native}"), common);
    }
}

pub fn code(i: usize) -> &'static str {
    LANGS.get(i).map_or("auto", |l| l.0)
}

/// Language name for a code a provider reported ("DE", "pt-BR", "zh").
pub fn name_of(code: &str) -> Option<&'static str> {
    let c = code.to_lowercase();
    let base = c.split(['-', '_']).next().unwrap_or("");
    LANGS.iter().skip(1).find(|l| l.0.to_lowercase() == c).or_else(|| LANGS.iter().skip(1).find(|l| l.0 == base || (base == "nb" && l.0 == "no"))).map(|l| l.1)
}
