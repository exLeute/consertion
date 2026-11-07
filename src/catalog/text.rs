//! Text formats. Converting A → B means `B.encode(A.decode(input))`.

use super::{Builder, Kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    Plain, Upper, Lower, Title, Sentence, Camel, Pascal, Snake, Kebab, Constant, Dot,
    Reversed, Rot13, Morse, Base64, Hex, Binary, DecimalBytes, Url, Html, CodePoints, Nato, Leet,
    Base32, Json, Sha256, Sha1, Md5, Crc32,
    /// Unix permissions: "755" ↔ "rwxr-xr-x" ↔ words. They decode to the octal digits.
    PermOctal, PermSymbolic, PermWords,
}

const FORMATS: &[(Format, &str, &str, &str)] = &[
    (Format::Plain, "Plain text", "abc", "text string"),
    (Format::Upper, "UPPERCASE", "ABC", "upper case caps"),
    (Format::Lower, "lowercase", "abc↓", "lower case"),
    (Format::Title, "Title Case", "Abc", "title case capitalize"),
    (Format::Sentence, "Sentence case", "Ab.", "sentence case"),
    (Format::Camel, "camelCase", "aB", "camel case"),
    (Format::Pascal, "PascalCase", "AB", "pascal case"),
    (Format::Snake, "snake_case", "a_b", "snake case underscore"),
    (Format::Kebab, "kebab-case", "a-b", "kebab case dash slug"),
    (Format::Constant, "CONSTANT_CASE", "A_B", "constant screaming"),
    (Format::Dot, "dot.case", "a.b", "dot case"),
    (Format::Reversed, "Reversed text", "cba", "reverse backwards"),
    (Format::Rot13, "ROT13", "rot13", "caesar cipher"),
    (Format::Morse, "Morse code", ".-", "morse"),
    (Format::Base64, "Base64", "b64", "base64 encode"),
    (Format::Hex, "Hex bytes", "0x", "hexadecimal utf-8 bytes"),
    (Format::Binary, "Binary bytes", "01", "binary utf-8 bits"),
    (Format::DecimalBytes, "Decimal bytes", "123", "ascii codes utf-8"),
    (Format::Url, "URL encoded", "%20", "percent encoding uri"),
    (Format::Html, "HTML entities", "&amp;", "html escape"),
    (Format::CodePoints, "Unicode code points", "U+", "unicode codepoint"),
    (Format::Nato, "NATO phonetic", "Alfa", "nato phonetic alphabet spelling"),
    (Format::Leet, "Leetspeak", "1337", "leet"),
    (Format::Base32, "Base32", "b32", "base32 rfc4648 totp encode"),
    (Format::Json, "JSON string", "\"…\"", "json escape quote string literal programming"),
    (Format::Sha256, "SHA-256 hash", "sha256", "hash checksum digest sha2 programming"),
    (Format::Sha1, "SHA-1 hash", "sha1", "hash checksum digest git programming"),
    (Format::Md5, "MD5 hash", "md5", "hash checksum digest programming"),
    (Format::Crc32, "CRC-32 checksum", "crc32", "checksum crc zip programming"),
];

const PERMS: &[(Format, &str, &str, &str)] = &[
    (Format::PermOctal, "Octal permissions", "755", "chmod unix linux file mode octal permissions"),
    (Format::PermSymbolic, "Symbolic permissions", "rwxr-xr-x", "chmod unix linux ls file mode permissions"),
    (Format::PermWords, "Permissions explained", "who/what", "chmod unix linux permissions meaning"),
];

pub fn register(b: &mut Builder) {
    b.category("text", "Text & Encoding", false);
    for &(f, name, sym, kw) in FORMATS {
        b.add(name, sym, Kind::Text(f), kw, true);
    }
    b.category("perms", "Unix Permissions", false);
    for &(f, name, sym, kw) in PERMS {
        b.add(name, sym, Kind::Text(f), kw, true);
    }
}

impl Format {
    pub fn can_decode(self) -> bool {
        !matches!(self, Format::Leet | Format::Sha256 | Format::Sha1 | Format::Md5 | Format::Crc32 | Format::PermWords)
    }

    /// Restrict input for formats with a fixed alphabet.
    pub fn accepts(self, s: &str) -> bool {
        let ok = |extra: &str, f: fn(char) -> bool| s.chars().all(|c| f(c) || extra.contains(c));
        match self {
            Format::Morse => ok(" /", |c| c == '.' || c == '-'),
            Format::Base64 => ok("+/=-_ \n", |c| c.is_ascii_alphanumeric()),
            Format::Hex => ok(" xX,", |c| c.is_ascii_hexdigit()),
            Format::Binary => ok(" ,", |c| c == '0' || c == '1'),
            Format::DecimalBytes => ok(" ,", |c| c.is_ascii_digit()),
            Format::CodePoints => ok(" uU+,", |c| c.is_ascii_hexdigit()),
            Format::Base32 => ok("= \n", |c| c.is_ascii_alphanumeric()),
            Format::PermOctal => s.len() <= 4 && ok("", |c| ('0'..='7').contains(&c)),
            Format::PermSymbolic => s.chars().count() <= 10 && ok("-", |c| "rwxstSTdl".contains(c)),
            _ => true,
        }
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            Format::Morse => "e.g. .... .. / - .... . .-. .",
            Format::Base64 => "e.g. SGVsbG8=",
            Format::Hex => "e.g. 48 65 6C 6C 6F",
            Format::Binary => "e.g. 01001000 01101001",
            Format::DecimalBytes => "e.g. 72 105",
            Format::CodePoints => "e.g. U+0048 U+0069",
            Format::Base32 => "e.g. JBSWY3DP",
            Format::Json => "e.g. \"line\\nbreak\"",
            Format::PermOctal => "e.g. 755 or 4755",
            Format::PermSymbolic => "e.g. rwxr-xr-x",
            _ => "type any text",
        }
    }

    /// Parse text in this format back into plain text.
    pub fn decode(self, s: &str) -> Result<String, String> {
        Ok(match self {
            Format::Camel | Format::Pascal | Format::Snake | Format::Kebab | Format::Constant | Format::Dot => {
                words(s).join(" ").to_lowercase()
            }
            Format::Reversed => s.chars().rev().collect(),
            Format::Rot13 => rot13(s),
            Format::Morse => morse_decode(s)?,
            Format::Base64 => utf8(base64_decode(s)?)?,
            Format::Hex => utf8(bytes_decode(s, 16)?)?,
            Format::Binary => utf8(bytes_decode(s, 2)?)?,
            Format::DecimalBytes => utf8(bytes_decode(s, 10)?)?,
            Format::Url => url_decode(s)?,
            Format::Html => html_decode(s),
            Format::CodePoints => codepoints_decode(s)?,
            Format::Nato => nato_decode(s)?,
            Format::Base32 => utf8(base32_decode(s)?)?,
            Format::Json => json_decode(s)?,
            Format::PermOctal => perm_octal(s)?,
            Format::PermSymbolic => perm_symbolic(s)?,
            Format::Leet | Format::Sha256 | Format::Sha1 | Format::Md5 | Format::Crc32 | Format::PermWords => {
                return Err("This format is output only".into())
            }
            Format::Plain | Format::Upper | Format::Lower | Format::Title | Format::Sentence => s.to_string(),
        })
    }

    pub fn encode(self, s: &str) -> String {
        let w = || words(s);
        match self {
            Format::Plain => s.to_string(),
            Format::Upper => s.to_uppercase(),
            Format::Lower => s.to_lowercase(),
            Format::Title => s.split(' ').map(capitalize).collect::<Vec<_>>().join(" "),
            Format::Sentence => capitalize(&s.to_lowercase()),
            Format::Camel => w().iter().enumerate().map(|(i, x)| if i == 0 { x.to_lowercase() } else { capitalize(x) }).collect(),
            Format::Pascal => w().iter().map(|x| capitalize(x)).collect(),
            Format::Snake => w().join("_").to_lowercase(),
            Format::Kebab => w().join("-").to_lowercase(),
            Format::Constant => w().join("_").to_uppercase(),
            Format::Dot => w().join(".").to_lowercase(),
            Format::Reversed => s.chars().rev().collect(),
            Format::Rot13 => rot13(s),
            Format::Morse => morse_encode(s),
            Format::Base64 => base64_encode(s.as_bytes()),
            Format::Hex => s.bytes().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(" "),
            Format::Binary => s.bytes().map(|b| format!("{b:08b}")).collect::<Vec<_>>().join(" "),
            Format::DecimalBytes => s.bytes().map(|b| b.to_string()).collect::<Vec<_>>().join(" "),
            Format::Url => url_encode(s),
            Format::Html => html_encode(s),
            Format::CodePoints => s.chars().map(|c| format!("U+{:04X}", c as u32)).collect::<Vec<_>>().join(" "),
            Format::Nato => nato_encode(s),
            Format::Leet => s.chars().map(leet).collect(),
            Format::Base32 => base32_encode(s.as_bytes()),
            Format::Json => json_encode(s),
            Format::Sha256 => super::hash::sha256(s.as_bytes()),
            Format::Sha1 => super::hash::sha1(s.as_bytes()),
            Format::Md5 => super::hash::md5(s.as_bytes()),
            Format::Crc32 => super::hash::crc32(s.as_bytes()),
            Format::PermOctal => s.to_string(),
            Format::PermSymbolic => perm_to_symbolic(s),
            Format::PermWords => perm_to_words(s),
        }
    }
}

/// Split into words on separators and lower→Upper boundaries ("fooBar_baz" → foo, Bar, baz).
fn words(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for c in s.chars() {
        if !c.is_alphanumeric() {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            prev_lower = false;
            continue;
        }
        if c.is_uppercase() && prev_lower {
            out.push(std::mem::take(&mut cur));
        }
        prev_lower = c.is_lowercase() || c.is_ascii_digit();
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c.flat_map(|x| x.to_lowercase())).collect(),
        None => String::new(),
    }
}

fn utf8(b: Vec<u8>) -> Result<String, String> {
    String::from_utf8(b).map_err(|_| "Decoded bytes are not valid UTF-8 text".into())
}

fn rot13(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' => (((c as u8 - b'a' + 13) % 26) + b'a') as char,
            'A'..='Z' => (((c as u8 - b'A' + 13) % 26) + b'A') as char,
            _ => c,
        })
        .collect()
}

const MORSE: &[(char, &str)] = &[
    ('A', ".-"), ('B', "-..."), ('C', "-.-."), ('D', "-.."), ('E', "."), ('F', "..-."), ('G', "--."),
    ('H', "...."), ('I', ".."), ('J', ".---"), ('K', "-.-"), ('L', ".-.."), ('M', "--"), ('N', "-."),
    ('O', "---"), ('P', ".--."), ('Q', "--.-"), ('R', ".-."), ('S', "..."), ('T', "-"), ('U', "..-"),
    ('V', "...-"), ('W', ".--"), ('X', "-..-"), ('Y', "-.--"), ('Z', "--.."), ('0', "-----"),
    ('1', ".----"), ('2', "..---"), ('3', "...--"), ('4', "....-"), ('5', "....."), ('6', "-...."),
    ('7', "--..."), ('8', "---.."), ('9', "----."), ('.', ".-.-.-"), (',', "--..--"), ('?', "..--.."),
    ('\'', ".----."), ('!', "-.-.--"), ('/', "-..-."), ('(', "-.--."), (')', "-.--.-"), ('&', ".-..."),
    (':', "---..."), (';', "-.-.-."), ('=', "-...-"), ('+', ".-.-."), ('-', "-....-"), ('_', "..--.-"),
    ('"', ".-..-."), ('$', "...-..-"), ('@', ".--.-."),
];

fn morse_encode(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            w.chars()
                .map(|c| MORSE.iter().find(|m| m.0 == c.to_ascii_uppercase()).map_or("?", |m| m.1))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

fn morse_decode(s: &str) -> Result<String, String> {
    s.split('/')
        .map(|w| {
            w.split_whitespace()
                .map(|code| {
                    MORSE.iter().find(|m| m.1 == code).map(|m| m.0).ok_or(format!("Unknown Morse code \"{code}\""))
                })
                .collect::<Result<String, String>>()
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|w| w.into_iter().filter(|w| !w.is_empty()).collect::<Vec<_>>().join(" "))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, &b)| acc | (b as u32) << (16 - 8 * i));
        for i in 0..4 {
            out.push(if i <= chunk.len() { B64[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    let mut bits = 0u32;
    let mut n = 0;
    let mut out = Vec::new();
    for c in s.chars().filter(|c| !c.is_whitespace() && *c != '=') {
        let v = match c {
            '-' => 62, // URL-safe alphabet
            '_' => 63,
            _ => B64.iter().position(|&b| b as char == c).ok_or("Invalid Base64 character")? as u32,
        };
        bits = bits << 6 | v;
        n += 6;
        if n >= 8 {
            n -= 8;
            out.push((bits >> n) as u8);
            bits &= (1 << n) - 1;
        }
    }
    Ok(out)
}

const B32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

fn base32_encode(bytes: &[u8]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(5) {
        let mut buf = [0u8; 5];
        buf[..chunk.len()].copy_from_slice(chunk);
        let n = buf.iter().fold(0u64, |a, &b| a << 8 | b as u64);
        let chars = (chunk.len() * 8).div_ceil(5);
        for i in 0..8 {
            out.push(if i < chars { B32[(n >> (35 - 5 * i) & 31) as usize] as char } else { '=' });
        }
    }
    out
}

fn base32_decode(s: &str) -> Result<Vec<u8>, String> {
    let (mut bits, mut n, mut out) = (0u64, 0, Vec::new());
    for c in s.chars().filter(|c| !c.is_whitespace() && *c != '=') {
        let v = B32.iter().position(|&b| b as char == c.to_ascii_uppercase()).ok_or("Invalid Base32 character")?;
        bits = (bits << 5 | v as u64) & 0xFFFF;
        n += 5;
        if n >= 8 {
            n -= 8;
            out.push((bits >> n) as u8);
        }
    }
    Ok(out)
}

fn json_encode(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out + "\""
}

fn json_decode(s: &str) -> Result<String, String> {
    let t = s.trim();
    let t = t.strip_prefix('"').and_then(|t| t.strip_suffix('"')).unwrap_or(t);
    let mut out = String::new();
    let mut it = t.chars();
    let hex4 = |it: &mut std::str::Chars| -> Result<u32, String> {
        let h: String = it.by_ref().take(4).collect();
        u32::from_str_radix(&h, 16).ok().filter(|_| h.len() == 4).ok_or("Bad \\u escape".into())
    };
    while let Some(c) = it.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match it.next().ok_or("Unfinished escape")? {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            'b' => '\u{8}',
            'f' => '\u{c}',
            '/' => '/',
            'u' => {
                let hi = hex4(&mut it)?;
                // A surrogate pair encodes one character outside the BMP.
                let code = if (0xD800..0xDC00).contains(&hi) && it.as_str().starts_with("\\u") {
                    it.nth(1);
                    0x10000 + ((hi - 0xD800) << 10) + (hex4(&mut it)? - 0xDC00)
                } else {
                    hi
                };
                char::from_u32(code).ok_or("Invalid code point")?
            }
            c => c,
        });
    }
    Ok(out)
}

fn perm_octal(s: &str) -> Result<String, String> {
    if s.is_empty() || s.len() > 4 || !s.chars().all(|c| ('0'..='7').contains(&c)) {
        return Err("Use 3 or 4 octal digits, e.g. 755".into());
    }
    Ok(format!("{:0>3}", s.trim_start_matches('0').chars().rev().take(4).collect::<String>().chars().rev().collect::<String>()))
}

fn perm_symbolic(s: &str) -> Result<String, String> {
    // An optional file-type letter ("drwxr-xr-x") is ignored.
    let s = if s.len() == 10 { &s[1..] } else { s };
    let c: Vec<char> = s.chars().collect();
    if c.len() != 9 {
        return Err("Use 9 characters, e.g. rwxr-xr-x".into());
    }
    let (mut special, mut digits) = (0, String::new());
    for (i, who) in c.chunks(3).enumerate() {
        let mut d = 0;
        for (j, &ch) in who.iter().enumerate() {
            let want = ['r', 'w', 'x'][j];
            match ch {
                '-' => {}
                ch if ch == want => d |= 4 >> j,
                's' | 't' if j == 2 => {
                    d |= 1;
                    special |= 4 >> i;
                }
                'S' | 'T' if j == 2 => special |= 4 >> i,
                _ => return Err(format!("\"{ch}\" can't be in that position")),
            }
        }
        digits.push(char::from(b'0' + d));
    }
    Ok(if special > 0 { format!("{special}{digits}") } else { digits })
}

fn perm_digits(octal: &str) -> Option<(u32, [u32; 3])> {
    let n = u32::from_str_radix(octal, 8).ok()?;
    Some((n >> 9 & 7, [n >> 6 & 7, n >> 3 & 7, n & 7]))
}

fn perm_to_symbolic(octal: &str) -> String {
    let Some((special, who)) = perm_digits(octal) else { return "?".into() };
    let mut out = String::new();
    for (i, d) in who.iter().enumerate() {
        out.push(if d & 4 != 0 { 'r' } else { '-' });
        out.push(if d & 2 != 0 { 'w' } else { '-' });
        let x = d & 1 != 0;
        out.push(match (special & (4 >> i) != 0, i == 2, x) {
            (true, false, true) => 's',
            (true, false, false) => 'S',
            (true, true, true) => 't',
            (true, true, false) => 'T',
            (false, _, true) => 'x',
            (false, _, false) => '-',
        });
    }
    out
}

fn perm_to_words(octal: &str) -> String {
    let Some((special, who)) = perm_digits(octal) else { return "?".into() };
    let mut parts: Vec<String> = ["owner", "group", "others"]
        .iter()
        .zip(who)
        .map(|(name, d)| {
            let can: Vec<&str> = [(4, "read"), (2, "write"), (1, "run")].iter().filter(|(b, _)| d & b != 0).map(|(_, w)| *w).collect();
            format!("{name}: {}", if can.is_empty() { "nothing".into() } else { can.join(", ") })
        })
        .collect();
    for (bit, name) in [(4, "setuid"), (2, "setgid"), (1, "sticky")] {
        if special & bit != 0 {
            parts.push(name.into());
        }
    }
    parts.join(" · ")
}

fn bytes_decode(s: &str, radix: u32) -> Result<Vec<u8>, String> {
    let s = s.replace("0x", " ").replace("0X", " ");
    let tokens: Vec<String> = if radix == 16 && !s.contains([' ', ',']) {
        // Unseparated hex like "48656C": split into pairs.
        if !s.len().is_multiple_of(2) {
            return Err("Hex needs an even number of digits".into());
        }
        s.as_bytes().chunks(2).map(|c| String::from_utf8_lossy(c).into_owned()).collect()
    } else {
        s.split([' ', ',']).filter(|t| !t.is_empty()).map(String::from).collect()
    };
    tokens
        .iter()
        .map(|t| u8::from_str_radix(t, radix).map_err(|_| format!("\"{t}\" is not a byte")))
        .collect()
}

fn url_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn url_decode(s: &str) -> Result<String, String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                let hex = s.get(i + 1..i + 3).ok_or("Incomplete % escape")?;
                out.push(u8::from_str_radix(hex, 16).map_err(|_| format!("Bad escape %{hex}"))?);
                i += 3;
                continue;
            }
            b'+' => out.push(b' '),
            c => out.push(c),
        }
        i += 1;
    }
    utf8(out)
}

fn html_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&#39;".into(),
            _ => c.to_string(),
        })
        .collect()
}

fn html_decode(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let Some(end) = rest.find(';').filter(|&e| e <= 10) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            _ if ent.starts_with("#x") || ent.starts_with("#X") => u32::from_str_radix(&ent[2..], 16).ok().and_then(char::from_u32),
            _ if ent.starts_with('#') => ent[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out + rest
}

fn codepoints_decode(s: &str) -> Result<String, String> {
    s.split([' ', ','])
        .map(|t| t.trim_start_matches(['U', 'u', '+']))
        .filter(|t| !t.is_empty())
        .map(|t| {
            u32::from_str_radix(t, 16).ok().and_then(char::from_u32).ok_or(format!("\"{t}\" is not a code point"))
        })
        .collect()
}

const NATO: [&str; 26] = [
    "Alfa", "Bravo", "Charlie", "Delta", "Echo", "Foxtrot", "Golf", "Hotel", "India", "Juliett", "Kilo",
    "Lima", "Mike", "November", "Oscar", "Papa", "Quebec", "Romeo", "Sierra", "Tango", "Uniform",
    "Victor", "Whiskey", "X-ray", "Yankee", "Zulu",
];
const NATO_DIGITS: [&str; 10] = ["Zero", "One", "Two", "Three", "Four", "Five", "Six", "Seven", "Eight", "Nine"];

fn nato_encode(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            w.chars()
                .map(|c| match c.to_ascii_uppercase() {
                    u @ 'A'..='Z' => NATO[(u as u8 - b'A') as usize].to_string(),
                    d @ '0'..='9' => NATO_DIGITS[(d as u8 - b'0') as usize].to_string(),
                    other => other.to_string(),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join(" / ")
}

fn nato_decode(s: &str) -> Result<String, String> {
    s.split('/')
        .map(|w| {
            w.split_whitespace()
                .map(|t| {
                    let find = |list: &[&str], base: u8| {
                        list.iter().position(|n| n.eq_ignore_ascii_case(t)).map(|i| (base + i as u8) as char)
                    };
                    find(&NATO, b'a')
                        .or_else(|| find(&NATO_DIGITS, b'0'))
                        .or_else(|| (t.eq_ignore_ascii_case("alpha")).then_some('a'))
                        .or_else(|| (t.eq_ignore_ascii_case("juliet")).then_some('j'))
                        .ok_or(format!("Unknown NATO word \"{t}\""))
                })
                .collect::<Result<String, String>>()
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|w| w.join(" "))
}

fn leet(c: char) -> char {
    match c.to_ascii_lowercase() {
        'a' => '4', 'e' => '3', 'i' => '1', 'o' => '0', 's' => '5', 't' => '7', 'b' => '8', 'g' => '9',
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Format::*;

    fn conv(from: Format, to: Format, s: &str) -> String {
        to.encode(&from.decode(s).unwrap())
    }

    #[test]
    fn cases() {
        assert_eq!(conv(Plain, Snake, "Hello big World"), "hello_big_world");
        assert_eq!(conv(Camel, Kebab, "parseHttpRequest2"), "parse-http-request2");
        assert_eq!(conv(Snake, Pascal, "my_var_name"), "MyVarName");
        assert_eq!(conv(Constant, Title, "MAX_VALUE"), "Max Value");
    }

    #[test]
    fn encodings_roundtrip() {
        for f in [Base64, Hex, Binary, DecimalBytes, Url, Html, CodePoints, Morse, Rot13, Reversed] {
            let s = if f == Morse { "SOS HELP 42" } else { "Héllo wörld & <tag> 100%" };
            assert_eq!(f.decode(&f.encode(s)).unwrap().to_uppercase(), s.to_uppercase(), "{f:?}");
        }
        assert_eq!(base64_encode(b"Man"), "TWFu");
        assert_eq!(base64_encode(b"Ma"), "TWE=");
        assert_eq!(Hex.decode("48656C6C6F").unwrap(), "Hello");
        assert_eq!(nato_encode("Hi 5"), "Hotel India / Five");
        assert_eq!(Nato.decode("hotel india / five").unwrap(), "hi 5");
        assert!(Morse.decode("...---...").is_err());
        assert!(Base64.decode("/w==").is_err()); // 0xFF is not UTF-8
        assert_eq!(base32_encode(b"foobar"), "MZXW6YTBOI======");
        assert_eq!(Base32.decode("MZXW6YTBOI======").unwrap(), "foobar");
        assert_eq!(Json.encode("a\"b\\c\n"), "\"a\\\"b\\\\c\\n\"");
        assert_eq!(Json.decode("\"\\ud83d\\ude00 \\u00e9\"").unwrap(), "😀 é");
        for t in ["", "x", "Héllo wörld"] {
            assert_eq!(Base32.decode(&Base32.encode(t)).unwrap(), t);
            assert_eq!(Json.decode(&Json.encode(t)).unwrap(), t);
        }
    }

    #[test]
    fn permissions() {
        assert_eq!(conv(PermOctal, PermSymbolic, "755"), "rwxr-xr-x");
        assert_eq!(conv(PermSymbolic, PermOctal, "drwxr-xr-x"), "755");
        assert_eq!(conv(PermOctal, PermSymbolic, "4755"), "rwsr-xr-x");
        assert_eq!(conv(PermSymbolic, PermOctal, "rwxrwxrwt"), "1777");
        assert_eq!(conv(PermOctal, PermWords, "640"), "owner: read, write · group: read · others: nothing");
        assert_eq!(conv(PermOctal, PermOctal, "0644"), "644");
        assert!(PermSymbolic.decode("rwxr-xr-").is_err());
    }
}
