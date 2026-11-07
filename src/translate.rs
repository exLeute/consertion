//! Machine translation through MyMemory (free, no key), DeepL or LibreTranslate (your own key).
//! Called only when the user translates; runs in a background thread like the rates fetch.

use crate::settings::{Provider, Settings};
use serde_json::{json, Value};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// Longest text one request may carry. MyMemory takes 500 bytes per query, so longer text is
/// split into lines and sentences; DeepL and LibreTranslate get it whole.
pub const MAX_CHARS: usize = 10_000;
const MYMEMORY_CHUNK: usize = 480;

#[derive(Clone, Debug)]
pub struct Request {
    pub provider: Provider,
    pub text: String,
    /// Our codes (`catalog::lang::LANGS`); `from` may be "auto".
    pub from: String,
    pub to: String,
    key: String,
    url: String,
    email: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Translation {
    pub text: String,
    /// Source language code the provider detected, when `from` was "auto".
    pub detected: Option<String>,
}

impl Request {
    pub fn new(s: &Settings, text: &str, from: &str, to: &str) -> Self {
        let key = match s.provider {
            Provider::DeepL => s.deepl_key.clone(),
            Provider::LibreTranslate => s.libre_key.clone(),
            Provider::MyMemory => String::new(),
        };
        Self { provider: s.provider, text: text.into(), from: from.into(), to: to.into(), key, url: s.libre_url.clone(), email: s.mymemory_email.clone() }
    }

    /// Settings problems found before anything is sent.
    pub fn check(&self) -> Result<(), String> {
        match self.provider {
            Provider::DeepL if self.key.trim().is_empty() => Err("DeepL needs your API key (Settings → Translation, F2)".into()),
            Provider::DeepL if deepl_code(&self.to, true).is_none() => Err(format!("DeepL can't translate into {}", lang_name(&self.to))),
            Provider::DeepL if self.from != "auto" && deepl_code(&self.from, false).is_none() => {
                Err(format!("DeepL can't translate from {}", lang_name(&self.from)))
            }
            Provider::LibreTranslate if !(self.url.starts_with("http://") || self.url.starts_with("https://")) => {
                Err("Set the LibreTranslate server URL (Settings → Translation, F2)".into())
            }
            _ if self.text.chars().count() > MAX_CHARS => Err(format!("Text is longer than {MAX_CHARS} characters")),
            _ => Ok(()),
        }
    }
}

fn lang_name(code: &str) -> &str {
    crate::catalog::lang::name_of(code).unwrap_or(code)
}

pub fn host(p: Provider, s: &Settings) -> String {
    match p {
        Provider::MyMemory => "api.mymemory.translated.net".into(),
        Provider::DeepL if s.deepl_key.trim().ends_with(":fx") => "api-free.deepl.com".into(),
        Provider::DeepL => "api.deepl.com".into(),
        Provider::LibreTranslate => s.libre_url.trim_start_matches("https://").trim_start_matches("http://").trim_end_matches('/').to_string(),
    }
}

pub fn provider_name(p: Provider) -> &'static str {
    match p {
        Provider::MyMemory => "MyMemory",
        Provider::DeepL => "DeepL",
        Provider::LibreTranslate => "LibreTranslate",
    }
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new().timeout(Duration::from_secs(20)).build()
}

/// Turn a ureq failure into something a person can act on.
fn http_error(provider: &str, e: ureq::Error) -> String {
    match e {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let detail = serde_json::from_str::<Value>(&body)
                .ok()
                .and_then(|v| v.get("message").or(v.get("error")).and_then(Value::as_str).map(String::from))
                .unwrap_or_default();
            let what = match code {
                401 | 403 => "rejected the API key",
                429 => "says too many requests — wait a moment",
                456 => "quota for this month is used up",
                _ => "returned an error",
            };
            let detail = if detail.is_empty() { String::new() } else { format!(": {}", detail.chars().take(160).collect::<String>()) };
            format!("{provider} {what} ({code}){detail}")
        }
        ureq::Error::Transport(t) => format!("Can't reach {provider}: {}", t.kind()),
    }
}

/// ureq is built without its JSON feature; serde_json does the (de)serializing.
fn read_json(resp: ureq::Response, provider: &str) -> Result<Value, String> {
    let body = resp.into_string().map_err(|_| format!("Unexpected reply from {provider}"))?;
    serde_json::from_str(&body).map_err(|_| format!("Unexpected reply from {provider}"))
}

fn post_json(req: ureq::Request, body: &Value, provider: &str) -> Result<Value, String> {
    let resp = req.set("Content-Type", "application/json").send_string(&body.to_string()).map_err(|e| http_error(provider, e))?;
    read_json(resp, provider)
}

pub fn translate(r: &Request) -> Result<Translation, String> {
    r.check()?;
    if r.text.trim().is_empty() {
        return Ok(Translation { text: String::new(), detected: None });
    }
    match r.provider {
        Provider::MyMemory => mymemory(r),
        Provider::DeepL => deepl(r),
        Provider::LibreTranslate => libre(r),
    }
}

pub fn translate_async(r: Request) -> Receiver<Result<Translation, String>> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(translate(&r));
    });
    rx
}

// ── MyMemory ─────────────────────────────────────────────────────────────────

/// Pieces of at most `max` bytes, split at sentence ends, then spaces, then anywhere.
fn chunks(line: &str, max: usize) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = line;
    while rest.len() > max {
        let mut cut = max;
        while !rest.is_char_boundary(cut) {
            cut -= 1;
        }
        let head = &rest[..cut];
        let at = head
            .rfind(['.', '!', '?', '。', ';'])
            .map(|i| i + head[i..].chars().next().map_or(1, char::len_utf8))
            .filter(|&i| i > max / 3)
            .or_else(|| head.rfind(' ').filter(|&i| i > 0))
            .unwrap_or(cut);
        out.push(&rest[..at]);
        rest = &rest[at..];
    }
    if !rest.is_empty() {
        out.push(rest);
    }
    out
}

fn mymemory(r: &Request) -> Result<Translation, String> {
    let from = if r.from == "auto" { "autodetect".to_string() } else { r.from.clone() };
    let pair = format!("{from}|{}", r.to);
    let agent = agent();
    let mut detected = None;
    // MyMemory joins lines, so each line goes separately and the layout survives.
    let mut lines = Vec::new();
    for line in r.text.split('\n') {
        let (body, indent) = (line.trim_start(), &line[..line.len() - line.trim_start().len()]);
        if body.trim().is_empty() {
            lines.push(line.to_string());
            continue;
        }
        let mut done = String::from(indent);
        for piece in chunks(body, MYMEMORY_CHUNK) {
            let mut req = agent.get("https://api.mymemory.translated.net/get").query("q", piece.trim()).query("langpair", &pair);
            if !r.email.trim().is_empty() {
                req = req.query("de", r.email.trim());
            }
            let v = read_json(req.call().map_err(|e| http_error("MyMemory", e))?, "MyMemory")?;
            let status = v["responseStatus"].as_i64().or_else(|| v["responseStatus"].as_str().and_then(|s| s.parse().ok())).unwrap_or(0);
            if v["quotaFinished"].as_bool() == Some(true) || status == 429 {
                return Err("MyMemory's free daily limit is used up — add an email in Settings or try tomorrow".into());
            }
            let text = v["responseData"]["translatedText"].as_str().unwrap_or_default();
            if status != 200 {
                let msg = v["responseDetails"].as_str().filter(|s| !s.is_empty()).unwrap_or(text);
                return Err(format!("MyMemory: {}", sentence_case(msg)));
            }
            if detected.is_none() {
                detected = v["responseData"]["detectedLanguage"].as_str().map(String::from);
            }
            if !done.trim().is_empty() {
                done.push(' ');
            }
            done.push_str(&unescape(text.trim()));
        }
        lines.push(done);
    }
    Ok(Translation { text: lines.join("\n"), detected })
}

/// MyMemory sometimes returns HTML entities ("it&#39;s").
fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        rest = &rest[i..];
        let end = rest.find(';').filter(|&e| e <= 10);
        let decoded = end.and_then(|e| match &rest[1..e] {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some(' '),
            n if n.starts_with("#x") => u32::from_str_radix(&n[2..], 16).ok().and_then(char::from_u32),
            n if n.starts_with('#') => n[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        });
        match (decoded, end) {
            (Some(c), Some(e)) => {
                out.push(c);
                rest = &rest[e + 1..];
            }
            _ => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out + rest
}

/// MyMemory shouts its errors ("INVALID TARGET LANGUAGE").
fn sentence_case(s: &str) -> String {
    if s.chars().any(|c| c.is_lowercase()) {
        return s.to_string();
    }
    let lower = s.to_lowercase();
    let mut c = lower.chars();
    c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
}

// ── DeepL ────────────────────────────────────────────────────────────────────

const DEEPL: &[&str] = &[
    "ar", "bg", "cs", "da", "de", "el", "en", "es", "et", "fi", "fr", "he", "hu", "id", "it", "ja", "ko", "lt", "lv", "no", "nl", "pl", "pt",
    "pt-BR", "ro", "ru", "sk", "sl", "sv", "th", "tr", "uk", "vi", "zh-CN", "zh-TW",
];

fn deepl_code(code: &str, target: bool) -> Option<String> {
    if !DEEPL.contains(&code) {
        return None;
    }
    Some(match (code, target) {
        ("en", true) => "EN-US".into(),
        ("pt", true) => "PT-PT".into(),
        ("pt-BR", true) => "PT-BR".into(),
        ("zh-CN", true) => "ZH-HANS".into(),
        ("zh-TW", true) => "ZH-HANT".into(),
        ("no", _) => "NB".into(),
        (c, _) => c.split('-').next().unwrap_or(c).to_uppercase(),
    })
}

fn deepl(r: &Request) -> Result<Translation, String> {
    let host = if r.key.trim().ends_with(":fx") { "api-free.deepl.com" } else { "api.deepl.com" };
    let mut body = json!({ "text": [r.text], "target_lang": deepl_code(&r.to, true) });
    if r.from != "auto" {
        body["source_lang"] = json!(deepl_code(&r.from, false));
    }
    let req = agent().post(&format!("https://{host}/v2/translate")).set("Authorization", &format!("DeepL-Auth-Key {}", r.key.trim()));
    let v = post_json(req, &body, "DeepL")?;
    let t = &v["translations"][0];
    let text = t["text"].as_str().ok_or("Unexpected reply from DeepL")?.to_string();
    Ok(Translation { text, detected: t["detected_source_language"].as_str().map(String::from) })
}

// ── LibreTranslate ───────────────────────────────────────────────────────────

fn libre_code(code: &str) -> &str {
    match code {
        "zh-CN" => "zh-Hans",
        "zh-TW" => "zh-Hant",
        "pt-BR" => "pt",
        "no" => "nb",
        c => c,
    }
}

fn libre(r: &Request) -> Result<Translation, String> {
    let url = format!("{}/translate", r.url.trim().trim_end_matches('/'));
    let mut body = json!({ "q": r.text, "source": libre_code(&r.from), "target": libre_code(&r.to), "format": "text" });
    if !r.key.trim().is_empty() {
        body["api_key"] = json!(r.key.trim());
    }
    let v = post_json(agent().post(&url), &body, "LibreTranslate")?;
    let text = v["translatedText"].as_str().ok_or_else(|| format!("LibreTranslate: {}", v["error"].as_str().unwrap_or("unexpected reply")))?.to_string();
    Ok(Translation { text, detected: v["detectedLanguage"]["language"].as_str().map(String::from) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_long_lines() {
        let s = "One sentence here. ".repeat(40);
        let parts = chunks(&s, 100);
        assert!(parts.iter().all(|p| p.len() <= 100));
        assert_eq!(parts.concat(), s, "nothing lost");
        assert!(parts[0].ends_with('.'), "cut at a sentence end: {:?}", parts[0]);
        let cyr = "й".repeat(300);
        assert_eq!(chunks(&cyr, 101).concat(), cyr, "never splits a character");
    }

    #[test]
    fn provider_codes() {
        assert_eq!(deepl_code("en", true).as_deref(), Some("EN-US"));
        assert_eq!(deepl_code("en", false).as_deref(), Some("EN"));
        assert_eq!(deepl_code("no", true).as_deref(), Some("NB"));
        assert_eq!(deepl_code("sw", true), None);
        assert_eq!(crate::catalog::lang::name_of("DE"), Some("German"));
        assert_eq!(crate::catalog::lang::name_of("pt-BR"), Some("Portuguese (Brazil)"));
        assert_eq!(unescape("it&#39;s &amp; &lt;b&gt; &bogus"), "it's & <b> &bogus");
        assert_eq!(sentence_case("'XX' IS AN INVALID TARGET LANGUAGE"), "'xx' is an invalid target language");
        let mut s = Settings::default();
        s.provider = Provider::DeepL;
        assert!(Request::new(&s, "hi", "en", "de").check().is_err(), "DeepL without a key");
        s.deepl_key = "abc:fx".into();
        assert!(Request::new(&s, "hi", "en", "de").check().is_ok());
        assert_eq!(host(Provider::DeepL, &s), "api-free.deepl.com");
    }
}
