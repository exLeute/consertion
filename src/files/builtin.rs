//! Conversions that need no installed tools: raster images, JSON/YAML/TOML/CSV/TSV, text
//! encodings and line endings, Markdown → HTML, and SRT ↔ WebVTT subtitles.

use super::{Job, FORMATS, O_DELIM, O_ENC, O_EOL, O_GRAY, O_INDENT, O_QUALITY, O_RESIZE};
use serde_json::{Map, Value};
use std::io::{BufWriter, Write};
use std::path::Path;

/// Text and data files bigger than this are refused (they're read whole into memory).
const MAX_TEXT: u64 = 256 * 1024 * 1024;

pub fn convert(job: &Job, tmp: &Path) -> Result<(), String> {
    let (a, b) = (FORMATS[job.from].ext, FORMATS[job.to].ext);
    match (a, b) {
        _ if FORMATS[job.from].media == super::Media::Image && FORMATS[job.to].media == super::Media::Image => image(job, tmp),
        ("md", "html") => {
            let title = job.input.file_stem().map_or(String::new(), |s| s.to_string_lossy().into_owned());
            write(tmp, markdown_to_html(&read_text(&job.input)?, &title).as_bytes())
        }
        ("srt" | "vtt", "srt" | "vtt") => {
            let text = read_text(&job.input)?;
            write(tmp, if b == "vtt" { srt_to_vtt(&text) } else { vtt_to_srt(&text) }.as_bytes())
        }
        (_, "txt") => {
            let text = read_text(&job.input)?;
            write(tmp, &encode_text(&text, job.opt(&O_ENC), job.opt(&O_EOL))?)
        }
        _ => {
            let value = read_data(a, &read_text(&job.input)?)?;
            write(tmp, write_data(b, value, job)?.as_bytes())
        }
    }
}

fn write(tmp: &Path, bytes: &[u8]) -> Result<(), String> {
    std::fs::write(tmp, bytes).map_err(|e| format!("Couldn't write the file: {e}"))
}

// ── Text ─────────────────────────────────────────────────────────────────────

/// Decode by BOM (UTF-8, UTF-16 LE/BE), else UTF-8, else Latin-1 (never fails).
pub fn decode(bytes: &[u8]) -> String {
    match bytes {
        [0xEF, 0xBB, 0xBF, rest @ ..] => String::from_utf8_lossy(rest).into_owned(),
        [0xFF, 0xFE, rest @ ..] => utf16(rest, u16::from_le_bytes),
        [0xFE, 0xFF, rest @ ..] => utf16(rest, u16::from_be_bytes),
        _ => match std::str::from_utf8(bytes) {
            Ok(s) => s.to_string(),
            Err(_) => bytes.iter().map(|&b| b as char).collect(),
        },
    }
}

fn utf16(b: &[u8], f: fn([u8; 2]) -> u16) -> String {
    let units: Vec<u16> = b.as_chunks::<2>().0.iter().map(|&c| f(c)).collect();
    String::from_utf16_lossy(&units)
}

fn read_text(p: &Path) -> Result<String, String> {
    let len = std::fs::metadata(p).map_err(|e| e.to_string())?.len();
    if len > MAX_TEXT {
        return Err(format!("File is too large for the built-in converter ({})", super::human_size(len)));
    }
    Ok(decode(&std::fs::read(p).map_err(|e| format!("Couldn't read the file: {e}"))?))
}

pub fn encode_text(text: &str, encoding: &str, eol: &str) -> Result<Vec<u8>, String> {
    let text = match eol {
        "Keep" => text.to_string(),
        e => {
            let lf = text.replace("\r\n", "\n");
            if e.starts_with("CRLF") { lf.replace('\n', "\r\n") } else { lf }
        }
    };
    Ok(match encoding {
        "UTF-8 with BOM" => [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat(),
        "UTF-16 LE" => [0xFF, 0xFE].into_iter().chain(text.encode_utf16().flat_map(u16::to_le_bytes)).collect(),
        "UTF-16 BE" => [0xFE, 0xFF].into_iter().chain(text.encode_utf16().flat_map(u16::to_be_bytes)).collect(),
        "Latin-1" => text
            .chars()
            .map(|c| u8::try_from(c as u32).map_err(|_| format!("\"{c}\" can't be written in Latin-1 — choose UTF-8")))
            .collect::<Result<_, _>>()?,
        _ => text.into_bytes(),
    })
}

// ── Data ─────────────────────────────────────────────────────────────────────

pub fn read_data(ext: &str, text: &str) -> Result<Value, String> {
    match ext {
        "json" => serde_json::from_str(text).map_err(|e| format!("Invalid JSON: {e}")),
        "yaml" => serde_yaml::from_str(text).map_err(|e| format!("Invalid YAML: {e}")),
        "toml" => toml::from_str::<toml::Value>(text).map(toml_to_json).map_err(|e| format!("Invalid TOML: {}", e.message())),
        "csv" | "tsv" => read_table(text, if ext == "tsv" { b'\t' } else { sniff_delimiter(text) }),
        _ => Err("Unsupported data format".into()),
    }
}

fn toml_to_json(v: toml::Value) -> Value {
    match v {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => i.into(),
        toml::Value::Float(f) => serde_json::Number::from_f64(f).map_or(Value::Null, Value::Number),
        toml::Value::Boolean(b) => b.into(),
        toml::Value::Datetime(d) => Value::String(d.to_string()),
        toml::Value::Array(a) => a.into_iter().map(toml_to_json).collect(),
        toml::Value::Table(t) => t.into_iter().map(|(k, v)| (k, toml_to_json(v))).collect::<Map<_, _>>().into(),
    }
}

/// European CSVs often use ";" — pick whichever of , ; | appears most in the header line.
fn sniff_delimiter(text: &str) -> u8 {
    let head = text.lines().next().unwrap_or("");
    b",;|".iter().copied().max_by_key(|d| head.bytes().filter(|b| b == d).count()).unwrap_or(b',')
}

/// Rows become objects keyed by the header; numbers and booleans are recognised.
fn read_table(text: &str, delim: u8) -> Result<Value, String> {
    let mut r = csv::ReaderBuilder::new().delimiter(delim).flexible(true).from_reader(text.as_bytes());
    let headers: Vec<String> = r.headers().map_err(|e| format!("Invalid table: {e}"))?.iter().map(String::from).collect();
    let mut rows = Vec::new();
    for rec in r.records() {
        let rec = rec.map_err(|e| format!("Invalid table: {e}"))?;
        let mut obj = Map::new();
        for (i, cell) in rec.iter().enumerate() {
            let key = headers.get(i).cloned().unwrap_or_else(|| format!("column{}", i + 1));
            obj.insert(key, guess(cell));
        }
        rows.push(Value::Object(obj));
    }
    Ok(Value::Array(rows))
}

fn guess(cell: &str) -> Value {
    let t = cell.trim();
    // Leading zeros ("007", ZIP codes) and "+1" stay text.
    let plain_number = !t.is_empty() && !t.starts_with('+') && !(t.len() > 1 && t.starts_with('0') && !t.starts_with("0."));
    if plain_number {
        if let Ok(i) = t.parse::<i64>() {
            return i.into();
        }
        if let Ok(f) = t.parse::<f64>()
            && f.is_finite()
            && t.chars().all(|c| c.is_ascii_digit() || ".-eE".contains(c))
        {
            return serde_json::Number::from_f64(f).map_or(Value::String(cell.into()), Value::Number);
        }
    }
    match t {
        "true" => true.into(),
        "false" => false.into(),
        _ => Value::String(cell.to_string()),
    }
}

pub fn write_data(ext: &str, v: Value, job: &Job) -> Result<String, String> {
    match ext {
        "json" => {
            let indent: &[u8] = match job.opt(&O_INDENT) {
                "Minified" => return Ok(v.to_string()),
                "4 spaces" => b"    ",
                "Tab" => b"\t",
                _ => b"  ",
            };
            let mut out = Vec::new();
            let mut ser = serde_json::Serializer::with_formatter(&mut out, serde_json::ser::PrettyFormatter::with_indent(indent));
            serde::Serialize::serialize(&v, &mut ser).map_err(|e| e.to_string())?;
            Ok(String::from_utf8(out).unwrap_or_default() + "\n")
        }
        "yaml" => serde_yaml::to_string(&v).map_err(|e| e.to_string()),
        "toml" => {
            // TOML needs a table at the top and has no null.
            let v = match strip_nulls(v)? {
                Value::Object(m) => Value::Object(m),
                other => Value::Object(Map::from_iter([("items".to_string(), other)])),
            };
            toml::to_string_pretty(&v).map_err(|e| format!("Can't write as TOML: {e}"))
        }
        "csv" | "tsv" => {
            let delim = match (ext, job.opt(&O_DELIM)) {
                ("tsv", _) => b'\t',
                (_, d) if d.starts_with("Semicolon") => b';',
                (_, d) if d.starts_with("Pipe") => b'|',
                _ => b',',
            };
            write_table(v, delim)
        }
        _ => Err("Unsupported data format".into()),
    }
}

fn strip_nulls(v: Value) -> Result<Value, String> {
    Ok(match v {
        Value::Object(m) => Value::Object(m.into_iter().filter(|(_, v)| !v.is_null()).map(|(k, v)| Ok((k, strip_nulls(v)?))).collect::<Result<_, String>>()?),
        Value::Array(a) if a.iter().any(Value::is_null) => return Err("TOML can't hold null values inside lists".into()),
        Value::Array(a) => Value::Array(a.into_iter().map(strip_nulls).collect::<Result<_, _>>()?),
        Value::Null => return Err("TOML can't hold a null value".into()),
        v => v,
    })
}

/// A list of records (objects) → rows under the union of their keys. Nested values become JSON.
fn write_table(v: Value, delim: u8) -> Result<String, String> {
    let rows = match v {
        Value::Array(a) => a,
        // {"rows": [...]} or any object holding exactly one list; else a single record.
        Value::Object(m) => match m.values().filter(|v| v.is_array()).count() {
            1 if m.len() == 1 => m.into_iter().next().map(|(_, v)| v.as_array().cloned().unwrap_or_default()).unwrap_or_default(),
            _ => vec![Value::Object(m)],
        },
        _ => return Err("A table needs a list of records".into()),
    };
    let cell = |v: &Value| match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    let mut w = csv::WriterBuilder::new().delimiter(delim).from_writer(Vec::new());
    let err = |e: csv::Error| e.to_string();
    if rows.iter().all(Value::is_array) {
        for r in &rows {
            w.write_record(r.as_array().unwrap().iter().map(cell)).map_err(err)?;
        }
    } else {
        let mut keys: Vec<String> = Vec::new();
        for r in &rows {
            let Value::Object(m) = r else { return Err("A table needs every row to be a record (an object)".into()) };
            for k in m.keys() {
                if !keys.contains(k) {
                    keys.push(k.clone());
                }
            }
        }
        w.write_record(&keys).map_err(err)?;
        for r in &rows {
            w.write_record(keys.iter().map(|k| r.get(k).map_or(String::new(), cell))).map_err(err)?;
        }
    }
    String::from_utf8(w.into_inner().map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

// ── Markdown ─────────────────────────────────────────────────────────────────

pub fn markdown_to_html(md: &str, title: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let opts = Options::ENABLE_TABLES | Options::ENABLE_FOOTNOTES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut body = String::new();
    html::push_html(&mut body, Parser::new_ext(md, opts));
    let title = title.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    format!(
        "<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{title}</title>\n<style>body{{max-width:46em;margin:2em auto;padding:0 1em;font:16px/1.6 system-ui,sans-serif}}pre,code{{background:#8881;border-radius:4px}}pre{{padding:.8em;overflow:auto}}table{{border-collapse:collapse}}td,th{{border:1px solid #8886;padding:.3em .6em}}img{{max-width:100%}}</style>\n</head>\n<body>\n{body}</body>\n</html>\n"
    )
}

// ── Subtitles ────────────────────────────────────────────────────────────────

fn is_timing(line: &str) -> bool {
    line.contains("-->")
}

pub fn srt_to_vtt(srt: &str) -> String {
    let mut out = String::from("WEBVTT\n\n");
    for line in srt.replace("\r\n", "\n").lines() {
        if is_timing(line) {
            out.push_str(&line.replace(',', "."));
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

pub fn vtt_to_srt(vtt: &str) -> String {
    let text = vtt.replace("\r\n", "\n");
    let mut out = String::new();
    let mut n = 0;
    for block in text.split("\n\n") {
        let lines: Vec<&str> = block.lines().collect();
        let Some(t) = lines.iter().position(|l| is_timing(l)) else { continue };
        n += 1;
        // "00:01.000 --> 00:02.500 align:start" → "00:00:01,000 --> 00:00:02,500"
        let mut parts = lines[t].split_whitespace();
        let (start, end) = (parts.next().unwrap_or(""), parts.nth(1).unwrap_or(""));
        let fix = |ts: &str| {
            let ts = ts.replace('.', ",");
            if ts.matches(':').count() == 1 { format!("00:{ts}") } else { ts }
        };
        out.push_str(&format!("{n}\n{} --> {}\n", fix(start), fix(end)));
        for l in &lines[t + 1..] {
            out.push_str(l);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

// ── Images ───────────────────────────────────────────────────────────────────

/// Resize choice → target size that fits (never enlarges).
pub fn fit_size(w: u32, h: u32, choice: &str) -> (u32, u32) {
    let scale = if let Some(p) = choice.strip_suffix('%') {
        p.parse::<f64>().unwrap_or(100.0) / 100.0
    } else if let Some(px) = choice.strip_prefix("Fit ").and_then(|s| s.strip_suffix(" px")) {
        let px = px.parse::<f64>().unwrap_or(f64::MAX);
        (px / w.max(h) as f64).min(1.0)
    } else {
        1.0
    };
    (((w as f64 * scale).round() as u32).max(1), ((h as f64 * scale).round() as u32).max(1))
}

fn image(job: &Job, tmp: &Path) -> Result<(), String> {
    use image::{codecs, DynamicImage, ImageFormat, ImageReader};
    let to = FORMATS[job.to].ext;
    // Sniff the content too: a ".png" that is really a JPEG still converts.
    let mut img = ImageReader::open(&job.input)
        .and_then(|r| r.with_guessed_format())
        .map_err(|e| format!("Couldn't read the image: {e}"))?
        .decode()
        .map_err(|e| format!("Couldn't decode the image: {e}"))?;
    let (w, h) = fit_size(img.width(), img.height(), job.opt(&O_RESIZE));
    let (w, h) = if to == "ico" { fit_size(w, h, "Fit 256 px") } else { (w, h) };
    if (w, h) != (img.width(), img.height()) {
        img = img.resize(w, h, image::imageops::FilterType::Lanczos3);
    }
    if job.opt(&O_GRAY) == "Grayscale" {
        img = if img.color().has_alpha() { DynamicImage::ImageLumaA8(img.to_luma_alpha8()) } else { DynamicImage::ImageLuma8(img.to_luma8()) };
    }
    // Formats without transparency: flatten onto white instead of black.
    if matches!(to, "jpg" | "ppm") && img.color().has_alpha() {
        let mut rgba = img.to_rgba8();
        for p in rgba.pixels_mut() {
            let a = p[3] as u32;
            for c in 0..3 {
                p[c] = ((p[c] as u32 * a + 255 * (255 - a)) / 255) as u8;
            }
            p[3] = 255;
        }
        img = DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(rgba).to_rgb8());
    }
    // Only PNG and TIFF store 16 bits per channel; the rest need 8.
    if !matches!(to, "png" | "tiff") && !matches!(img, DynamicImage::ImageRgb8(_) | DynamicImage::ImageRgba8(_) | DynamicImage::ImageLuma8(_)) {
        img = if img.color().has_alpha() { DynamicImage::ImageRgba8(img.to_rgba8()) } else { DynamicImage::ImageRgb8(img.to_rgb8()) };
    }
    let file = std::fs::File::create(tmp).map_err(|e| format!("Couldn't write the file: {e}"))?;
    let mut out = BufWriter::new(file);
    let r = match to {
        "jpg" => {
            let q = job.opt(&O_QUALITY).parse().unwrap_or(90);
            img.write_with_encoder(codecs::jpeg::JpegEncoder::new_with_quality(&mut out, q))
        }
        "webp" => img.write_with_encoder(codecs::webp::WebPEncoder::new_lossless(&mut out)),
        _ => {
            let fmt = match to {
                "png" => ImageFormat::Png,
                "gif" => ImageFormat::Gif,
                "bmp" => ImageFormat::Bmp,
                "tiff" => ImageFormat::Tiff,
                "ico" => ImageFormat::Ico,
                "tga" => ImageFormat::Tga,
                "ppm" => ImageFormat::Pnm,
                "qoi" => ImageFormat::Qoi,
                _ => return Err("Unsupported image format".into()),
            };
            img.write_to(&mut out, fmt)
        }
    };
    r.map_err(|e| format!("Couldn't encode the image: {e}"))?;
    out.flush().map_err(|e| format!("Couldn't write the file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files::{by_ext, Backend};

    fn job(from: &str, to: &str, opts: Vec<(&'static str, usize)>) -> Job {
        Job { input: "x".into(), output: "y".into(), from: by_ext(from).unwrap(), to: by_ext(to).unwrap(), backend: Backend::Builtin, opts, overwrite: false }
    }

    #[test]
    fn data_round_trips() {
        let csv = "name;age;zip\nAda;36;01234\nLin;7.5;\n";
        assert!(write_data("json", read_data("json", r#"{"b":1,"a":2}"#).unwrap(), &job("json", "json", vec![("data.indent", 3)])).unwrap() == r#"{"b":1,"a":2}"#, "key order kept");
        let v = read_data("csv", csv).unwrap();
        assert_eq!(v[0]["age"], 36);
        assert_eq!(v[0]["zip"], "01234", "leading zeros stay text");
        assert_eq!(v[1]["age"], 7.5);
        let j = job("csv", "json", vec![]);
        let json = write_data("json", v.clone(), &j).unwrap();
        assert!(json.contains("\"name\": \"Ada\""));
        let toml = write_data("toml", v.clone(), &j).unwrap();
        assert!(toml.contains("[[items]]"), "{toml}");
        let back = read_data("toml", &toml).unwrap();
        assert_eq!(back["items"][0]["name"], "Ada");
        let yaml = write_data("yaml", v.clone(), &j).unwrap();
        assert_eq!(read_data("yaml", &yaml).unwrap(), v);
        let table = write_data("csv", read_data("json", r#"[{"a":1,"b":{"c":2}},{"a":"x,y"}]"#).unwrap(), &j).unwrap();
        assert_eq!(table, "a,b\n1,\"{\"\"c\"\":2}\"\n\"x,y\",\n");
        let min = write_data("json", v, &job("csv", "json", vec![("data.indent", 3)])).unwrap();
        assert!(!min.contains('\n'));
        assert!(read_data("json", "{oops").is_err());
    }

    #[test]
    fn text_and_subtitles() {
        assert_eq!(decode(&[0xFF, 0xFE, b'h', 0, b'i', 0]), "hi");
        assert_eq!(decode(&[0xE9]), "é", "Latin-1 fallback");
        assert_eq!(encode_text("a\nb", "UTF-16 BE", "CRLF (Windows)").unwrap(), [0xFE, 0xFF, 0, b'a', 0, b'\r', 0, b'\n', 0, b'b']);
        assert!(encode_text("й", "Latin-1", "Keep").is_err());
        let srt = "1\r\n00:00:01,000 --> 00:00:02,500\r\nHello\r\n\r\n2\r\n00:00:03,000 --> 00:00:04,000\r\nWorld\r\n";
        let vtt = srt_to_vtt(srt);
        assert!(vtt.starts_with("WEBVTT\n\n1\n00:00:01.000 --> 00:00:02.500\nHello"));
        let back = vtt_to_srt("WEBVTT\n\nNOTE hi\n\n00:01.000 --> 00:02.500 align:start\nHello\n");
        assert_eq!(back, "1\n00:00:01,000 --> 00:00:02,500\nHello\n\n");
        assert!(markdown_to_html("# Hi\n\n|a|b|\n|-|-|\n|1|2|", "<t>").contains("<table>"));
        assert_eq!(fit_size(4000, 2000, "Fit 1920 px"), (1920, 960));
        assert_eq!(fit_size(100, 50, "Fit 1920 px"), (100, 50), "never enlarges");
        assert_eq!(fit_size(100, 50, "50%"), (50, 25));
    }

    #[test]
    fn image_conversion() {
        let dir = std::env::temp_dir().join(format!("consertion-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let src = dir.join("a.png");
        let mut img = image::RgbaImage::new(300, 200);
        img.put_pixel(0, 0, image::Rgba([255, 0, 0, 0]));
        img.save(&src).unwrap();
        // 16-bit source: QOI, GIF, BMP… need 8 bits.
        let deep = dir.join("deep.png");
        image::DynamicImage::ImageRgba16(image::ImageBuffer::from_pixel(8, 8, image::Rgba([65535u16, 0, 0, 65535]))).save(&deep).unwrap();
        let mut j = job("png", "qoi", vec![]);
        j.input = deep;
        convert(&j, &dir.join("deep.qoi")).unwrap();
        for (to, opts) in [("jpg", vec![("image.resize", 2)]), ("ico", vec![]), ("webp", vec![]), ("bmp", vec![("image.gray", 1)]), ("gif", vec![]), ("tga", vec![])] {
            let mut j = job("png", to, opts);
            j.input = src.clone();
            let out = dir.join(format!("b.{to}"));
            convert(&j, &out).unwrap();
            let back = image::open(&out).unwrap();
            match to {
                "jpg" => {
                    assert_eq!((back.width(), back.height()), (150, 100));
                    assert!(back.to_rgb8().get_pixel(0, 0)[1] > 200, "transparent → white");
                }
                "ico" => assert!(back.width() <= 256),
                _ => assert_eq!(back.width(), 300),
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
