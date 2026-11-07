//! File conversion: images, audio, video, documents, spreadsheets, slides, data, text and
//! subtitles. Pure-Rust converters handle what they can; installed tools (ffmpeg, ImageMagick,
//! LibreOffice, pandoc, poppler, rsvg) do the rest, started directly (never through a shell).
//! Only conversions something on this computer can do are offered.
//!
//! Safety: the result is written to a hidden temporary file next to the output and renamed into
//! place only when the converter succeeded, so a failed or cancelled run never leaves a broken
//! file, and an existing file is only replaced after the user confirmed it.
//!
//! Adding a format: a row in `FORMATS`, then the backends that read/write it in `backends`.

pub mod builtin;
pub mod tools;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Media {
    Image,
    Audio,
    Video,
    Document,
    Sheet,
    Slides,
    Data,
    Text,
    Subtitle,
}

pub struct Fmt {
    /// Canonical extension; also the format's id.
    pub ext: &'static str,
    pub name: &'static str,
    /// Other extensions that mean the same format.
    pub alias: &'static [&'static str],
    pub media: Media,
    pub kw: &'static str,
}

const fn f(ext: &'static str, name: &'static str, alias: &'static [&'static str], media: Media, kw: &'static str) -> Fmt {
    Fmt { ext, name, alias, media, kw }
}

use Media::*;

pub const FORMATS: &[Fmt] = &[
    f("png", "PNG image", &[], Image, "picture photo screenshot lossless transparent"),
    f("jpg", "JPEG image", &["jpeg", "jpe", "jfif"], Image, "jpg picture photo camera"),
    f("webp", "WebP image", &[], Image, "picture photo web google"),
    f("gif", "GIF image / animation", &[], Image, "animated animation picture meme"),
    f("bmp", "BMP bitmap", &["dib"], Image, "windows bitmap picture"),
    f("tiff", "TIFF image", &["tif"], Image, "scan print picture"),
    f("ico", "ICO icon", &[], Image, "favicon windows icon"),
    f("avif", "AVIF image", &[], Image, "av1 picture photo modern"),
    f("heic", "HEIC / HEIF image", &["heif"], Image, "iphone apple photo high efficiency"),
    f("jxl", "JPEG XL image", &[], Image, "jpegxl picture photo"),
    f("tga", "TGA (Targa) image", &[], Image, "targa texture game"),
    f("ppm", "PPM / PNM image", &["pnm", "pgm", "pbm"], Image, "netpbm portable pixmap"),
    f("qoi", "QOI image", &[], Image, "quite ok image"),
    f("svg", "SVG vector image", &[], Image, "vector scalable graphics logo"),
    f("psd", "Photoshop PSD", &[], Image, "adobe photoshop layers"),
    f("exr", "OpenEXR image", &[], Image, "hdr high dynamic range"),
    f("mp3", "MP3 audio", &[], Audio, "music song sound mpeg"),
    f("wav", "WAV audio", &["wave"], Audio, "sound uncompressed pcm"),
    f("flac", "FLAC audio", &[], Audio, "lossless music sound"),
    f("ogg", "Ogg Vorbis audio", &["oga"], Audio, "music sound vorbis"),
    f("opus", "Opus audio", &[], Audio, "voice music sound"),
    f("m4a", "M4A (AAC) audio", &[], Audio, "aac itunes apple music sound"),
    f("aac", "AAC audio", &[], Audio, "advanced audio coding sound"),
    f("aiff", "AIFF audio", &["aif"], Audio, "apple uncompressed sound"),
    f("wma", "WMA audio", &[], Audio, "windows media audio sound"),
    f("ac3", "AC-3 (Dolby) audio", &[], Audio, "dolby digital surround sound"),
    f("mp4", "MP4 video", &["m4v"], Video, "movie film clip h264 mpeg4"),
    f("mkv", "MKV (Matroska) video", &[], Video, "matroska movie film"),
    f("webm", "WebM video", &[], Video, "vp9 movie web clip"),
    f("mov", "MOV (QuickTime) video", &[], Video, "quicktime apple iphone movie"),
    f("avi", "AVI video", &[], Video, "movie film divx xvid"),
    f("flv", "FLV (Flash) video", &[], Video, "flash movie"),
    f("wmv", "WMV video", &[], Video, "windows media movie"),
    f("mpg", "MPEG video", &["mpeg"], Video, "mpeg2 dvd movie"),
    f("ts", "MPEG-TS video", &["mts", "m2ts"], Video, "transport stream camcorder"),
    f("3gp", "3GP video", &[], Video, "phone mobile movie"),
    f("ogv", "Ogg (Theora) video", &[], Video, "theora movie"),
    f("pdf", "PDF document", &[], Document, "portable document print acrobat"),
    f("docx", "Word document (DOCX)", &[], Document, "microsoft word office"),
    f("doc", "Word 97 document (DOC)", &[], Document, "microsoft word office old"),
    f("odt", "OpenDocument text (ODT)", &[], Document, "libreoffice writer"),
    f("rtf", "Rich Text Format", &[], Document, "wordpad rich text"),
    f("html", "HTML web page", &["htm"], Document, "web page website"),
    f("epub", "EPUB e-book", &[], Document, "ebook book reader"),
    f("md", "Markdown", &["markdown"], Document, "markdown readme text"),
    f("rst", "reStructuredText", &[], Document, "rest sphinx docs"),
    f("tex", "LaTeX", &[], Document, "latex tex paper"),
    f("txt", "Text file", &["text", "log"], Text, "text file encoding utf8 utf16 latin1 line endings"),
    f("xlsx", "Excel spreadsheet (XLSX)", &[], Sheet, "microsoft excel office table"),
    f("xls", "Excel 97 spreadsheet (XLS)", &[], Sheet, "microsoft excel office old"),
    f("ods", "OpenDocument spreadsheet (ODS)", &[], Sheet, "libreoffice calc table"),
    f("pptx", "PowerPoint (PPTX)", &[], Slides, "microsoft powerpoint presentation slides"),
    f("ppt", "PowerPoint 97 (PPT)", &[], Slides, "microsoft powerpoint presentation old"),
    f("odp", "OpenDocument presentation (ODP)", &[], Slides, "libreoffice impress slides"),
    f("json", "JSON data", &[], Data, "javascript object data api"),
    f("yaml", "YAML data", &["yml"], Data, "config data"),
    f("toml", "TOML data", &[], Data, "config cargo data"),
    f("csv", "CSV table", &[], Data, "comma separated values table spreadsheet"),
    f("tsv", "TSV table", &["tab"], Data, "tab separated values table"),
    f("srt", "SRT subtitles", &[], Subtitle, "subrip captions"),
    f("vtt", "WebVTT subtitles", &[], Subtitle, "web captions"),
    f("ass", "ASS / SSA subtitles", &["ssa"], Subtitle, "advanced substation captions"),
];

/// `Kind::File(PICK)`: the "Browse for a file…" entry, which detects the format.
pub const PICK: usize = usize::MAX;

pub fn register(b: &mut crate::catalog::Builder) {
    use crate::catalog::Kind;
    b.category("files", "File formats", false);
    b.add("Browse for a file…", "open", Kind::File(PICK), "file open browse convert document photo picture video audio music", true);
    for (i, f) in FORMATS.iter().enumerate() {
        let exts: Vec<&str> = std::iter::once(f.ext).chain(f.alias.iter().copied()).collect();
        let common = matches!(f.media, Image | Audio | Video | Document) && i < 50;
        b.add(f.name, &format!(".{}", f.ext), Kind::File(i), &format!("file convert {} {}", exts.join(" "), f.kw), common);
    }
}

pub fn by_ext(ext: &str) -> Option<usize> {
    let e = ext.trim_start_matches('.').to_lowercase();
    FORMATS.iter().position(|f| f.ext == e || f.alias.contains(&e.as_str()))
}

pub fn of_path(p: &Path) -> Option<usize> {
    by_ext(p.extension()?.to_str()?)
}

fn is(i: usize, exts: &[&str]) -> bool {
    exts.contains(&FORMATS[i].ext)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    Builtin,
    Magick,
    Ffmpeg,
    LibreOffice,
    Pandoc,
    Poppler,
    Rsvg,
}

impl Backend {
    pub fn name(self) -> &'static str {
        match self {
            Backend::Builtin => "Built-in",
            Backend::Magick => "ImageMagick",
            Backend::Ffmpeg => "ffmpeg",
            Backend::LibreOffice => "LibreOffice",
            Backend::Pandoc => "pandoc",
            Backend::Poppler => "Poppler",
            Backend::Rsvg => "rsvg-convert",
        }
    }
}

// What each converter reads and writes.
const RASTER_IN: &[&str] = &["png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "tga", "ppm", "qoi"];
const RASTER_OUT: &[&str] = &["png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "tga", "ppm", "qoi"];
const MAGICK_IN: &[&str] = &["png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "tga", "ppm", "qoi", "avif", "heic", "jxl", "svg", "psd", "exr", "pdf"];
const MAGICK_OUT: &[&str] = &["png", "jpg", "webp", "gif", "bmp", "tiff", "ico", "tga", "ppm", "qoi", "avif", "heic", "jxl", "exr", "pdf"];
const FF_IMAGE: &[&str] = &["png", "jpg", "webp", "bmp", "tiff", "gif"];
const DATA: &[&str] = &["json", "yaml", "toml", "csv", "tsv"];
const TEXTLIKE: &[&str] = &["txt", "md", "csv", "tsv", "json", "yaml", "toml", "html", "srt", "vtt", "ass", "rst", "tex", "svg"];
const WRITER: &[&str] = &["docx", "doc", "odt", "rtf", "html", "txt"];
const WRITER_OUT: &[&str] = &["pdf", "docx", "doc", "odt", "rtf", "html", "txt", "epub"];
const CALC: &[&str] = &["xlsx", "xls", "ods", "csv"];
const CALC_OUT: &[&str] = &["xlsx", "xls", "ods", "csv", "pdf", "html"];
const IMPRESS: &[&str] = &["pptx", "ppt", "odp"];
const IMPRESS_OUT: &[&str] = &["pptx", "ppt", "odp", "pdf"];
const PANDOC_IN: &[&str] = &["md", "docx", "odt", "epub", "html", "rst", "tex", "txt", "rtf"];
const PANDOC_OUT: &[&str] = &["md", "docx", "odt", "epub", "html", "rst", "tex", "txt", "rtf"];
const LOSSY_AUDIO: &[&str] = &["mp3", "ogg", "opus", "m4a", "aac", "wma", "ac3"];
const LOSSY_IMAGE: &[&str] = &["jpg", "webp", "avif", "heic", "jxl"];
const VECTOR: &[&str] = &["svg", "pdf"];

/// Converters that can turn format `a` into `b` on this computer, best first.
pub fn backends(a: usize, b: usize) -> Vec<Backend> {
    let (fa, fb) = (&FORMATS[a], &FORMATS[b]);
    let t = tools::tools();
    let mut out = Vec::new();
    let mut add = |ok: bool, be: Backend| {
        if ok && !out.contains(&be) {
            out.push(be);
        }
    };
    let same = a == b;
    // Built-in: images, data, text, Markdown → HTML, SRT ↔ WebVTT.
    add(is(a, RASTER_IN) && is(b, RASTER_OUT), Backend::Builtin);
    add(is(a, DATA) && is(b, DATA) && (!same || is(a, &["json", "yaml"])), Backend::Builtin);
    add(fb.ext == "txt" && is(a, TEXTLIKE), Backend::Builtin);
    add(fa.ext == "md" && fb.ext == "html", Backend::Builtin);
    add(is(a, &["srt", "vtt"]) && is(b, &["srt", "vtt"]) && !same, Backend::Builtin);
    // Vector input renders best with rsvg / poppler.
    add(t.rsvg.is_some() && fa.ext == "svg" && is(b, &["png", "pdf"]), Backend::Rsvg);
    add(t.pdftoppm.is_some() && fa.ext == "pdf" && is(b, &["png", "jpg"]), Backend::Poppler);
    add(t.pdftotext.is_some() && fa.ext == "pdf" && fb.ext == "txt", Backend::Poppler);
    let magick_ok = t.magick.is_some()
        && is(a, MAGICK_IN)
        && is(b, MAGICK_OUT)
        && tools::magick_can(fa.ext, false)
        && tools::magick_can(fb.ext, true)
        && (fa.ext != "pdf" || (t.gs && fb.media == Image))
        && !(fa.ext == "pdf" && fb.ext == "pdf");
    add(magick_ok, Backend::Magick);
    if t.ffmpeg.is_some() {
        let av = |m: Media| matches!(m, Audio | Video);
        add(fa.media == Audio && fb.media == Audio, Backend::Ffmpeg);
        add(fa.media == Video && (fb.media == Video || fb.media == Audio), Backend::Ffmpeg);
        add(fa.media == Video && (fb.ext == "gif" || is(b, &["png", "jpg", "webp"])), Backend::Ffmpeg);
        add(fa.ext == "gif" && fb.media == Video, Backend::Ffmpeg);
        add(is(a, FF_IMAGE) && is(b, FF_IMAGE) && fa.ext != "gif" && !av(fb.media), Backend::Ffmpeg);
        add(fa.media == Subtitle && fb.media == Subtitle && !same, Backend::Ffmpeg);
        add(is(a, &["mkv", "mp4", "webm", "mov"]) && fb.media == Subtitle, Backend::Ffmpeg);
    }
    if t.soffice.is_some() && !same {
        add(is(a, WRITER) && is(b, WRITER_OUT), Backend::LibreOffice);
        add(is(a, CALC) && is(b, CALC_OUT), Backend::LibreOffice);
        add(is(a, IMPRESS) && is(b, IMPRESS_OUT), Backend::LibreOffice);
    }
    add(t.pandoc.is_some() && is(a, PANDOC_IN) && is(b, PANDOC_OUT) && !same, Backend::Pandoc);
    out
}

pub fn convertible(a: usize, b: usize) -> bool {
    a != PICK && b != PICK && !backends(a, b).is_empty()
}

/// Tools that would unlock conversions from this format, for the "nothing available" hint.
pub fn suggest(a: usize) -> &'static str {
    match FORMATS[a].media {
        Audio | Video => "ffmpeg",
        Image => "ImageMagick (or ffmpeg)",
        Document if FORMATS[a].ext == "pdf" => "poppler-utils or ImageMagick + Ghostscript",
        Document | Sheet | Slides => "LibreOffice or pandoc",
        _ => "ffmpeg",
    }
}

// ── Options ──────────────────────────────────────────────────────────────────

pub struct Opt {
    /// Persisted under this key, so the last choice is remembered per kind of option.
    pub key: &'static str,
    pub label: &'static str,
    pub choices: &'static [&'static str],
    pub default: usize,
}

const fn o(key: &'static str, label: &'static str, choices: &'static [&'static str], default: usize) -> Opt {
    Opt { key, label, choices, default }
}

pub const O_RESIZE: Opt = o("image.resize", "Resize", &["Original", "75%", "50%", "25%", "Fit 3840 px", "Fit 1920 px", "Fit 1280 px", "Fit 640 px", "Fit 256 px"], 0);
pub const O_QUALITY: Opt = o("image.quality", "Quality", &["100", "95", "90", "85", "75", "60", "40"], 2);
pub const O_GRAY: Opt = o("image.gray", "Colors", &["Keep", "Grayscale"], 0);
pub const O_STRIP: Opt = o("image.strip", "Metadata (EXIF, GPS)", &["Keep", "Remove"], 0);
pub const O_DPI: Opt = o("vector.dpi", "Render at", &["96 DPI", "150 DPI", "300 DPI", "600 DPI"], 2);
pub const O_PAGE: Opt = o("pdf.page", "Page", &["1", "2", "3", "4", "5", "6", "7", "8", "9", "10"], 0);
pub const O_PDF_TEXT: Opt = o("pdf.layout", "Layout", &["Keep columns", "Reading order"], 0);
pub const O_BITRATE: Opt = o("audio.bitrate", "Bitrate", &["320 kbps", "256 kbps", "192 kbps", "160 kbps", "128 kbps", "96 kbps", "64 kbps"], 2);
pub const O_RATE: Opt = o("audio.rate", "Sample rate", &["Original", "48000 Hz", "44100 Hz", "32000 Hz", "22050 Hz", "16000 Hz"], 0);
pub const O_CHANNELS: Opt = o("audio.channels", "Channels", &["Original", "Stereo", "Mono"], 0);
pub const O_VQUALITY: Opt = o("video.quality", "Quality", &["Best (large file)", "High", "Balanced", "Small", "Tiny"], 2);
pub const O_SPEED: Opt = o("video.speed", "Encoding speed", &["Slow (smaller file)", "Medium", "Fast", "Very fast"], 1);
pub const O_RES: Opt = o("video.res", "Resolution", &["Original", "2160p", "1440p", "1080p", "720p", "480p", "360p"], 0);
pub const O_FPS: Opt = o("video.fps", "Frame rate", &["Original", "60 fps", "30 fps", "24 fps"], 0);
pub const O_AUDIO: Opt = o("video.audio", "Audio track", &["Keep", "Remove"], 0);
pub const O_GIF_W: Opt = o("gif.width", "Width", &["Original", "800 px", "640 px", "480 px", "320 px"], 3);
pub const O_GIF_FPS: Opt = o("gif.fps", "Frame rate", &["24 fps", "15 fps", "10 fps"], 1);
pub const O_FRAME: Opt = o("video.frame", "Frame at", &["Start", "1 s", "5 s", "10 s", "30 s", "1 min"], 1);
pub const O_INDENT: Opt = o("data.indent", "Indent", &["2 spaces", "4 spaces", "Tab", "Minified"], 0);
pub const O_DELIM: Opt = o("data.delim", "Delimiter", &["Comma (,)", "Semicolon (;)", "Pipe (|)"], 0);
pub const O_ENC: Opt = o("text.encoding", "Encoding", &["UTF-8", "UTF-8 with BOM", "UTF-16 LE", "UTF-16 BE", "Latin-1"], 0);
pub const O_EOL: Opt = o("text.eol", "Line endings", &["Keep", "LF (Linux, macOS)", "CRLF (Windows)"], 0);

/// Settings that apply to this conversion with this converter (only what it supports).
pub fn options(a: usize, b: usize, be: Backend) -> Vec<&'static Opt> {
    let (fa, fb) = (&FORMATS[a], &FORMATS[b]);
    let mut v: Vec<&'static Opt> = Vec::new();
    match (be, fb.media) {
        (Backend::Ffmpeg, Image) if fb.ext == "gif" && fa.media == Video => v.extend([&O_GIF_W, &O_GIF_FPS]),
        (Backend::Ffmpeg, Image) => {
            if fa.media == Video {
                v.push(&O_FRAME);
            }
            v.push(&O_RESIZE);
            if is(b, &["jpg", "webp"]) {
                v.push(&O_QUALITY);
            }
        }
        (Backend::Ffmpeg, Audio) => {
            if is(b, LOSSY_AUDIO) {
                v.push(&O_BITRATE);
            }
            if fb.ext != "opus" {
                v.push(&O_RATE);
            }
            v.push(&O_CHANNELS);
        }
        (Backend::Ffmpeg, Video) => {
            v.push(&O_VQUALITY);
            if is(b, &["mp4", "mov", "mkv", "3gp", "ts", "flv", "webm"]) {
                v.push(&O_SPEED);
            }
            v.extend([&O_RES, &O_FPS]);
            if fa.media == Video {
                v.push(&O_AUDIO);
            }
        }
        (Backend::Builtin, Image) => {
            v.push(&O_RESIZE);
            if fb.ext == "jpg" {
                v.push(&O_QUALITY);
            }
            v.push(&O_GRAY);
        }
        (Backend::Magick, Image | Document) => {
            if is(a, VECTOR) {
                v.push(&O_DPI);
            }
            v.push(&O_RESIZE);
            if is(b, LOSSY_IMAGE) {
                v.push(&O_QUALITY);
            }
            v.extend([&O_GRAY, &O_STRIP]);
        }
        (Backend::Poppler, Image) => {
            v.extend([&O_PAGE, &O_DPI]);
            if fb.ext == "jpg" {
                v.push(&O_QUALITY);
            }
        }
        (Backend::Poppler, Text) => v.push(&O_PDF_TEXT),
        (Backend::Rsvg, _) => v.push(&O_DPI),
        (Backend::Builtin, Data) if fb.ext == "json" => v.push(&O_INDENT),
        (Backend::Builtin, Data) if fb.ext == "csv" => v.push(&O_DELIM),
        (Backend::Builtin, Text) => v.extend([&O_ENC, &O_EOL]),
        _ => {}
    }
    v
}

// ── Paths ────────────────────────────────────────────────────────────────────

/// A path as typed or pasted: quotes, `file://` URLs (drag and drop), `~`, escaped spaces.
pub fn parse_path(s: &str) -> PathBuf {
    let mut s = s.trim().to_string();
    for q in ['"', '\''] {
        if s.len() >= 2 && s.starts_with(q) && s.ends_with(q) {
            s = s[1..s.len() - 1].to_string();
        }
    }
    if let Some(rest) = s.strip_prefix("file://") {
        s = percent_decode(rest);
        // file:///C:/x on Windows
        if cfg!(windows) && s.starts_with('/') && s.as_bytes().get(2) == Some(&b':') {
            s.remove(0);
        }
    }
    if !cfg!(windows) && s.contains("\\ ") {
        s = s.replace("\\ ", " ");
    }
    let p = match s.strip_prefix('~') {
        Some(rest) if rest.is_empty() || rest.starts_with('/') || rest.starts_with('\\') => match dirs::home_dir() {
            Some(h) => h.join(rest.trim_start_matches(['/', '\\'])),
            None => PathBuf::from(&s),
        },
        _ => PathBuf::from(&s),
    };
    if p.is_relative() && !s.is_empty() {
        std::env::current_dir().map(|d| d.join(&p)).unwrap_or(p)
    } else {
        p
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Some(v) = std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// "~/Pictures/a.png" instead of "/home/me/Pictures/a.png".
pub fn display_path(p: &Path) -> String {
    if let Some(h) = dirs::home_dir()
        && let Ok(rest) = p.strip_prefix(&h)
    {
        return Path::new("~").join(rest).to_string_lossy().into_owned();
    }
    p.to_string_lossy().into_owned()
}

/// Next to the input, same name, new extension; never an existing file or the input itself.
pub fn default_output(input: &Path, to: usize) -> PathBuf {
    let ext = FORMATS[to].ext;
    let dir = input.parent().map(Path::to_path_buf).unwrap_or_default();
    let stem = input.file_stem().map_or("converted".into(), |s| s.to_string_lossy().into_owned());
    let first = dir.join(format!("{stem}.{ext}"));
    if !first.exists() && first != input {
        return first;
    }
    (1..1000).map(|n| dir.join(format!("{stem}-{n}.{ext}"))).find(|p| !p.exists()).unwrap_or(first)
}

pub fn human_size(n: u64) -> String {
    const U: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1000.0 && i < U.len() - 1 {
        v /= 1000.0;
        i += 1;
    }
    if i == 0 { format!("{n} B") } else { format!("{v:.1} {}", U[i]) }
}

// ── Running a conversion ─────────────────────────────────────────────────────

pub struct Job {
    pub input: PathBuf,
    pub output: PathBuf,
    pub from: usize,
    pub to: usize,
    pub backend: Backend,
    /// Chosen index per option key.
    pub opts: Vec<(&'static str, usize)>,
    /// The user confirmed replacing an existing output file.
    pub overwrite: bool,
}

impl Job {
    /// The chosen choice of an option (its default when not offered).
    pub fn opt(&self, o: &Opt) -> &'static str {
        let i = self.opts.iter().find(|(k, _)| *k == o.key).map_or(o.default, |x| x.1);
        o.choices[i.min(o.choices.len() - 1)]
    }
}

#[derive(Debug)]
pub struct Done {
    pub output: PathBuf,
    pub bytes: u64,
    pub secs: f32,
}

pub enum Event {
    /// 0.0–1.0, when the converter reports it (ffmpeg).
    Progress(f32),
    Done(Result<Done, String>),
}

pub struct Running {
    pub rx: Receiver<Event>,
    cancel: Arc<AtomicBool>,
    pub started: Instant,
    pub progress: Option<f32>,
}

impl Running {
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

pub fn start(job: Job) -> Running {
    let (tx, rx) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    std::thread::spawn(move || {
        let r = run(&job, &flag, &tx);
        let _ = tx.send(Event::Done(r));
    });
    Running { rx, cancel, started: Instant::now(), progress: None }
}

/// Checks before anything runs. Also used by the screen to explain problems early.
pub fn validate(input: &Path, output: &Path, overwrite: bool) -> Result<(), String> {
    let meta = std::fs::metadata(input).map_err(|_| "Input file not found".to_string())?;
    if !meta.is_file() {
        return Err("The input is a folder — choose a file".into());
    }
    if output.file_name().is_none_or(|n| n.is_empty()) {
        return Err("Choose an output file name".into());
    }
    match output.parent() {
        Some(d) if !d.as_os_str().is_empty() && !d.is_dir() => return Err("The output folder doesn't exist".into()),
        _ => {}
    }
    if output.is_dir() {
        return Err("The output is a folder — add a file name".into());
    }
    let same = output == input || matches!((input.canonicalize(), output.canonicalize()), (Ok(a), Ok(b)) if a == b);
    if same {
        return Err("The output would replace the input — choose another name".into());
    }
    if output.exists() && !overwrite {
        return Err("The output file already exists".into());
    }
    Ok(())
}

/// Hidden temp file next to the output, with the target format's extension (tools pick the
/// format from it).
fn temp_path(output: &Path, to: usize) -> PathBuf {
    use std::sync::atomic::AtomicUsize;
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let stem = output.file_stem().map_or("out".into(), |s| s.to_string_lossy().into_owned());
    output.with_file_name(format!(".{stem}.consertion-{}-{n}.{}", std::process::id(), FORMATS[to].ext))
}

fn run(job: &Job, cancel: &AtomicBool, tx: &Sender<Event>) -> Result<Done, String> {
    let t0 = Instant::now();
    validate(&job.input, &job.output, job.overwrite)?;
    let tmp = temp_path(&job.output, job.to);
    let result = match job.backend {
        Backend::Builtin => builtin::convert(job, &tmp),
        _ => tools::convert(job, &tmp, cancel, tx),
    }
    .and_then(|()| {
        if cancel.load(Ordering::Relaxed) {
            return Err("Cancelled".to_string());
        }
        let bytes = std::fs::metadata(&tmp).map_err(|_| "The converter didn't write a file".to_string())?.len();
        if bytes == 0 {
            return Err("The converter wrote an empty file".into());
        }
        // Re-check: the file may have appeared while converting.
        if job.output.exists() && !job.overwrite {
            return Err("The output file appeared while converting; nothing was replaced".into());
        }
        std::fs::rename(&tmp, &job.output).map_err(|e| format!("Couldn't save the file: {e}"))?;
        Ok(Done { output: job.output.clone(), bytes, secs: t0.elapsed().as_secs_f32() })
    });
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_paths() {
        assert_eq!(FORMATS[by_ext("JPEG").unwrap()].ext, "jpg");
        assert_eq!(of_path(Path::new("/a/b.Yml")).map(|i| FORMATS[i].ext), Some("yaml"));
        let mut exts: Vec<&str> = FORMATS.iter().flat_map(|f| std::iter::once(f.ext).chain(f.alias.iter().copied())).collect();
        exts.sort();
        assert!(exts.windows(2).all(|w| w[0] != w[1]), "an extension belongs to one format");
        // "/tmp" isn't absolute on Windows (no drive letter).
        if !cfg!(windows) {
            assert_eq!(parse_path("'/tmp/a b.png'"), PathBuf::from("/tmp/a b.png"));
            assert_eq!(parse_path("file:///tmp/a%20b.png"), PathBuf::from("/tmp/a b.png"));
            assert_eq!(parse_path("/tmp/a\\ b.png"), PathBuf::from("/tmp/a b.png"));
        }
        assert!(parse_path("~/x").starts_with(dirs::home_dir().unwrap()));
        assert_eq!(human_size(999), "999 B");
        assert_eq!(human_size(1_250_000), "1.2 MB");
        // Built-in conversions don't depend on installed tools.
        let [png, jpg, json, csv, md, html] = ["png", "jpg", "json", "csv", "md", "html"].map(|e| by_ext(e).unwrap());
        assert_eq!(backends(png, jpg)[0], Backend::Builtin);
        assert!(backends(json, csv).contains(&Backend::Builtin));
        assert!(backends(md, html).contains(&Backend::Builtin));
        assert!(options(png, jpg, Backend::Builtin).iter().any(|o| o.key == "image.quality"));
        assert!(!options(png, by_ext("bmp").unwrap(), Backend::Builtin).iter().any(|o| o.key == "image.quality"));
    }
}
