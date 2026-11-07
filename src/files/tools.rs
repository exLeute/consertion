//! Installed converters: found on PATH (plus LibreOffice's usual install folders), started
//! with argument lists — no shell, so file names can never be run as commands. Paths are
//! absolute, so a name starting with "-" can't be read as an option either.

use super::{Backend, Event, Job, Media, FORMATS};
use super::{O_AUDIO, O_BITRATE, O_CHANNELS, O_DPI, O_FPS, O_FRAME, O_GIF_FPS, O_GIF_W, O_GRAY, O_PAGE, O_PDF_TEXT, O_QUALITY, O_RATE, O_RES, O_RESIZE, O_SPEED, O_STRIP, O_VQUALITY};
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Default)]
pub struct Tools {
    pub ffmpeg: Option<PathBuf>,
    pub ffprobe: Option<PathBuf>,
    pub magick: Option<PathBuf>,
    pub soffice: Option<PathBuf>,
    pub pandoc: Option<PathBuf>,
    pub pdftotext: Option<PathBuf>,
    pub pdftoppm: Option<PathBuf>,
    pub rsvg: Option<PathBuf>,
    /// Ghostscript, which ImageMagick needs to read PDFs.
    pub gs: bool,
}

fn which(name: &str) -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat"] } else { &[""] };
    std::env::split_paths(&std::env::var_os("PATH")?).flat_map(|d| exts.iter().map(move |e| d.join(format!("{name}{e}")))).find(|p| p.is_file())
}

/// Detected once per run (a PATH lookup, no programs started).
pub fn tools() -> &'static Tools {
    static T: OnceLock<Tools> = OnceLock::new();
    T.get_or_init(|| {
        let office_dirs = [
            "/Applications/LibreOffice.app/Contents/MacOS/soffice",
            r"C:\Program Files\LibreOffice\program\soffice.exe",
            r"C:\Program Files (x86)\LibreOffice\program\soffice.exe",
        ];
        Tools {
            ffmpeg: which("ffmpeg"),
            ffprobe: which("ffprobe"),
            // ImageMagick 6 is "convert" — but on Windows that name is a disk tool.
            magick: which("magick").or_else(|| if cfg!(windows) { None } else { which("convert") }),
            soffice: which("soffice").or_else(|| which("libreoffice")).or_else(|| office_dirs.iter().map(PathBuf::from).find(|p| p.is_file())),
            pandoc: which("pandoc"),
            pdftotext: which("pdftotext"),
            pdftoppm: which("pdftoppm"),
            rsvg: which("rsvg-convert"),
            gs: which("gs").or_else(|| which("gswin64c")).is_some(),
        }
    })
}

/// Can this ImageMagick read (or write) the format? Asked once (`magick -list format`, ~0.1 s).
pub fn magick_can(ext: &str, write: bool) -> bool {
    static LIST: OnceLock<Option<Vec<(String, bool, bool)>>> = OnceLock::new();
    let list = LIST.get_or_init(|| {
        let out = Command::new(tools().magick.as_ref()?).args(["-list", "format"]).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        let text = String::from_utf8_lossy(&out.stdout);
        let rows: Vec<(String, bool, bool)> = text
            .lines()
            .filter_map(|l| {
                let mut it = l.split_whitespace();
                let name = it.next()?.trim_end_matches('*').to_lowercase();
                let mode = it.nth(1)?;
                (mode.len() == 3 && mode.chars().all(|c| "rw+-".contains(c))).then(|| (name, mode.starts_with('r'), mode.as_bytes()[1] == b'w'))
            })
            .collect();
        (!rows.is_empty()).then_some(rows)
    });
    let name = match ext {
        "jpg" => "jpeg",
        e => e,
    };
    match list {
        Some(rows) => rows.iter().any(|(n, r, w)| n == name && if write { *w } else { *r }),
        // Couldn't ask: assume the common formats work.
        None => ["png", "jpeg", "gif", "bmp", "tiff", "webp", "ico", "tga", "ppm"].contains(&name),
    }
}

/// ImageMagick reads with the format fixed by our detection ("PNG:file"), so a file can't be
/// interpreted as a script format by its content.
fn magick_coder(ext: &str) -> String {
    match ext {
        "jpg" => "JPEG".into(),
        "ppm" => "PNM".into(),
        e => e.to_uppercase(),
    }
}

fn os(s: impl Into<OsString>) -> OsString {
    s.into()
}

fn num_prefix(s: &str) -> String {
    s.chars().take_while(|c| c.is_ascii_digit()).collect()
}

pub fn convert(job: &Job, tmp: &Path, cancel: &AtomicBool, tx: &Sender<Event>) -> Result<(), String> {
    let t = tools();
    let missing = || format!("{} isn't installed", job.backend.name());
    match job.backend {
        Backend::Ffmpeg => {
            let exe = t.ffmpeg.as_ref().ok_or_else(missing)?;
            let duration = if matches!(FORMATS[job.from].media, Media::Audio | Media::Video) { probe_duration(&job.input) } else { None };
            run(exe, &ffmpeg_args(job, tmp), cancel, duration.map(|d| (d, tx)), "ffmpeg")
        }
        Backend::Magick => run(t.magick.as_ref().ok_or_else(missing)?, &magick_args(job, tmp), cancel, None, "ImageMagick"),
        Backend::Rsvg => {
            let zoom = num_prefix(job.opt(&O_DPI)).parse::<f64>().unwrap_or(96.0) / 96.0;
            let fmt = FORMATS[job.to].ext;
            let args = vec![os("-f"), os(fmt), os("-z"), os(format!("{zoom}")), os("-o"), tmp.into(), job.input.clone().into()];
            run(t.rsvg.as_ref().ok_or_else(missing)?, &args, cancel, None, "rsvg-convert")
        }
        Backend::Poppler if FORMATS[job.to].ext == "txt" => {
            let mut args = vec![os("-enc"), os("UTF-8")];
            if job.opt(&O_PDF_TEXT) == "Keep columns" {
                args.push(os("-layout"));
            }
            args.extend([job.input.clone().into(), tmp.into()]);
            run(t.pdftotext.as_ref().ok_or_else(missing)?, &args, cancel, None, "pdftotext")?;
            let text = std::fs::read(tmp).unwrap_or_default();
            if text.iter().all(|b| b.is_ascii_whitespace() || *b == 0x0c) {
                return Err("This PDF has no text layer (scanned pages need OCR) — convert it to an image instead".into());
            }
            Ok(())
        }
        Backend::Poppler => {
            let jpg = FORMATS[job.to].ext == "jpg";
            let page = job.opt(&O_PAGE);
            let mut args = vec![os(if jpg { "-jpeg" } else { "-png" }), os("-r"), os(num_prefix(job.opt(&O_DPI))), os("-f"), os(page), os("-l"), os(page), os("-singlefile")];
            if jpg {
                args.extend([os("-jpegopt"), os(format!("quality={}", job.opt(&O_QUALITY)))]);
            }
            // pdftoppm adds the extension itself.
            args.extend([job.input.clone().into(), tmp.with_extension("").into()]);
            run(t.pdftoppm.as_ref().ok_or_else(missing)?, &args, cancel, None, "pdftoppm")
        }
        Backend::Pandoc => {
            let reader = match FORMATS[job.from].ext {
                "md" | "txt" => "markdown",
                "tex" => "latex",
                e => e,
            };
            let writer = match FORMATS[job.to].ext {
                "md" => "markdown",
                "tex" => "latex",
                "txt" => "plain",
                e => e,
            };
            let mut args = vec![job.input.clone().into(), os("-f"), os(reader), os("-t"), os(writer), os("-o"), tmp.into()];
            if matches!(writer, "html" | "latex" | "rtf") {
                args.push(os("--standalone"));
            }
            run(t.pandoc.as_ref().ok_or_else(missing)?, &args, cancel, None, "pandoc")
        }
        Backend::LibreOffice => libreoffice(job, tmp, cancel),
        Backend::Builtin => unreachable!("built-in conversions don't use tools"),
    }
}

// ── ffmpeg ───────────────────────────────────────────────────────────────────

fn probe_duration(input: &Path) -> Option<f64> {
    let out = Command::new(tools().ffprobe.as_ref()?)
        .args(["-v", "error", "-show_entries", "format=duration", "-of", "default=nw=1:nk=1"])
        .arg(input)
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout).trim().parse().ok().filter(|d: &f64| *d > 0.0)
}

/// ffmpeg scale filter for a resize choice; dimensions stay even (most video codecs need it).
fn ff_scale(choice: &str, even: bool) -> Option<String> {
    let r = if even { "trunc(%/2)*2" } else { "%" };
    let dim = |e: &str| r.replace('%', e);
    if let Some(p) = choice.strip_suffix('%') {
        let k = p.parse::<f64>().ok()? / 100.0;
        return Some(format!("scale='{}':'{}'", dim(&format!("iw*{k}")), dim(&format!("ih*{k}"))));
    }
    if let Some(px) = choice.strip_prefix("Fit ").and_then(|s| s.strip_suffix(" px")) {
        return Some(format!("scale='min({px},iw)':'min({px},ih)':force_original_aspect_ratio=decrease{}", if even { ":force_divisible_by=2" } else { "" }));
    }
    if let Some(h) = choice.strip_suffix('p') {
        return Some(format!("scale=-2:'{}'", dim(&format!("min({h},ih)"))));
    }
    even.then(|| format!("scale='{}':'{}'", dim("iw"), dim("ih")))
}

fn ffmpeg_args(job: &Job, tmp: &Path) -> Vec<OsString> {
    let (fa, fb) = (&FORMATS[job.from], &FORMATS[job.to]);
    let mut a: Vec<OsString> = ["-hide_banner", "-nostdin", "-v", "error", "-y"].map(os).into();
    // A still from a video: seek before opening (fast).
    if fa.media == Media::Video && fb.media == Media::Image && fb.ext != "gif" {
        let at = match job.opt(&O_FRAME) {
            "Start" => "0",
            "1 min" => "60",
            s => s.trim_end_matches(" s"),
        };
        a.extend([os("-ss"), os(at)]);
    }
    a.extend([os("-i"), job.input.clone().into()]);
    let mut vf: Vec<String> = Vec::new();
    match fb.media {
        Media::Image if fb.ext == "gif" => {
            let fps = num_prefix(job.opt(&O_GIF_FPS));
            let w = match num_prefix(job.opt(&O_GIF_W)).as_str() {
                "" => "iw".to_string(),
                n => format!("'min({n},iw)'"),
            };
            let chain = format!("fps={fps},scale={w}:-1:flags=lanczos,split[s0][s1];[s0]palettegen[p];[s1][p]paletteuse");
            a.extend([os("-vf"), os(chain), os("-loop"), os("0")]);
        }
        Media::Image => {
            a.extend([os("-frames:v"), os("1"), os("-update"), os("1")]);
            if let Some(s) = ff_scale(job.opt(&O_RESIZE), false) {
                vf.push(s);
            }
            let q: u32 = job.opt(&O_QUALITY).parse().unwrap_or(90);
            match fb.ext {
                // mjpeg: 1 (best) … 31
                "jpg" => a.extend([os("-q:v"), os(((100 - q.min(100)) * 31 / 100).max(1).to_string())]),
                "webp" => a.extend([os("-quality"), os(q.to_string())]),
                _ => {}
            }
        }
        Media::Audio => {
            a.push(os("-vn"));
            let codec = match fb.ext {
                "mp3" => "libmp3lame",
                "ogg" => "libvorbis",
                "opus" => "libopus",
                "m4a" | "aac" => "aac",
                "wma" => "wmav2",
                "ac3" => "ac3",
                "flac" => "flac",
                "aiff" => "pcm_s16be",
                _ => "pcm_s16le",
            };
            a.extend([os("-c:a"), os(codec)]);
            if super::LOSSY_AUDIO.contains(&fb.ext) {
                a.extend([os("-b:a"), os(format!("{}k", num_prefix(job.opt(&O_BITRATE))))]);
            }
            if let r @ 1.. = num_prefix(job.opt(&O_RATE)).parse::<u32>().unwrap_or(0)
                && fb.ext != "opus"
            {
                a.extend([os("-ar"), os(r.to_string())]);
            }
            match job.opt(&O_CHANNELS) {
                "Stereo" => a.extend([os("-ac"), os("2")]),
                "Mono" => a.extend([os("-ac"), os("1")]),
                _ => {}
            }
        }
        Media::Video => {
            let level = O_VQUALITY.choices.iter().position(|c| *c == job.opt(&O_VQUALITY)).unwrap_or(2);
            let speed = O_SPEED.choices.iter().position(|c| *c == job.opt(&O_SPEED)).unwrap_or(1);
            let (v, aud): (Vec<String>, Vec<&str>) = match fb.ext {
                "webm" => (
                    vec!["-c:v", "libvpx-vp9", "-b:v", "0", "-row-mt", "1", "-deadline", "good"]
                        .into_iter()
                        .map(String::from)
                        .chain(["-crf".into(), [20, 28, 33, 38, 45][level].to_string(), "-cpu-used".into(), [1, 2, 4, 6][speed].to_string()])
                        .collect(),
                    vec!["-c:a", "libopus", "-b:a", "128k"],
                ),
                "avi" => (vec!["-c:v".into(), "mpeg4".into(), "-q:v".into(), [1, 3, 5, 8, 12][level].to_string()], vec!["-c:a", "libmp3lame", "-b:a", "192k"]),
                "wmv" => (vec!["-c:v".into(), "wmv2".into(), "-q:v".into(), [1, 3, 5, 8, 12][level].to_string()], vec!["-c:a", "wmav2", "-b:a", "192k"]),
                "mpg" => (vec!["-c:v".into(), "mpeg2video".into(), "-q:v".into(), [1, 3, 5, 8, 12][level].to_string()], vec!["-c:a", "mp2", "-b:a", "192k"]),
                "ogv" => (vec!["-c:v".into(), "libtheora".into(), "-q:v".into(), [10, 8, 6, 4, 2][level].to_string()], vec!["-c:a", "libvorbis", "-q:a", "5"]),
                _ => (
                    vec!["-c:v", "libx264", "-pix_fmt", "yuv420p"]
                        .into_iter()
                        .map(String::from)
                        .chain(["-crf".into(), [16, 20, 23, 28, 33][level].to_string(), "-preset".into(), ["slow", "medium", "fast", "veryfast"][speed].into()])
                        .collect(),
                    vec!["-c:a", "aac", "-b:a", "160k"],
                ),
            };
            a.extend(v.into_iter().map(os));
            if fa.media == Media::Video && job.opt(&O_AUDIO) == "Remove" || fa.media != Media::Video {
                a.push(os("-an"));
            } else {
                a.extend(aud.into_iter().map(os));
            }
            if matches!(fb.ext, "mp4" | "mov") {
                a.extend([os("-movflags"), os("+faststart")]);
            }
            if let fps @ 1.. = num_prefix(job.opt(&O_FPS)).parse::<u32>().unwrap_or(0) {
                vf.push(format!("fps={fps}"));
            }
            vf.extend(ff_scale(job.opt(&O_RES), true));
        }
        Media::Subtitle if fa.media == Media::Video => a.extend([os("-map"), os("0:s:0")]),
        _ => {}
    }
    if !vf.is_empty() {
        a.extend([os("-vf"), os(vf.join(","))]);
    }
    a.extend([os("-progress"), os("pipe:1"), os("-nostats"), tmp.into()]);
    a
}

// ── ImageMagick ──────────────────────────────────────────────────────────────

fn magick_args(job: &Job, tmp: &Path) -> Vec<OsString> {
    let (fa, fb) = (FORMATS[job.from].ext, FORMATS[job.to].ext);
    let mut a = Vec::new();
    if matches!(fa, "svg" | "pdf") {
        a.extend([os("-density"), os(num_prefix(job.opt(&O_DPI)))]);
    }
    // Only the first frame/page/layer, unless the target can hold an animation.
    let keep_frames = matches!(fa, "gif" | "webp" | "tiff") && matches!(fb, "gif" | "webp" | "tiff" | "pdf");
    let mut spec = OsString::from(format!("{}:", magick_coder(fa)));
    spec.push(job.input.as_os_str());
    if !keep_frames {
        spec.push("[0]");
    }
    a.push(spec);
    a.push(os("-auto-orient"));
    match job.opt(&O_RESIZE) {
        "Original" => {}
        r if r.ends_with('%') => a.extend([os("-resize"), os(r)]),
        r => {
            let px = num_prefix(r.trim_start_matches("Fit "));
            a.extend([os("-resize"), os(format!("{px}x{px}>"))]);
        }
    }
    if job.opt(&O_GRAY) == "Grayscale" {
        a.extend([os("-colorspace"), os("Gray")]);
    }
    if job.opt(&O_STRIP) == "Remove" {
        a.push(os("-strip"));
    }
    if super::LOSSY_IMAGE.contains(&fb) {
        a.extend([os("-quality"), os(job.opt(&O_QUALITY))]);
    }
    match fb {
        "jpg" | "ppm" | "bmp" => a.extend(["-background", "white", "-alpha", "remove", "-alpha", "off"].map(os)),
        "ico" => a.extend([os("-define"), os("icon:auto-resize=256,128,64,48,32,16")]),
        _ => {}
    }
    a.push(tmp.into());
    a
}

// ── LibreOffice ──────────────────────────────────────────────────────────────

fn file_url(p: &Path) -> String {
    let s = p.to_string_lossy().replace('\\', "/");
    let enc: String = s
        .bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"/-_.~:".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect();
    if enc.starts_with('/') { format!("file://{enc}") } else { format!("file:///{enc}") }
}

fn libreoffice(job: &Job, tmp: &Path, cancel: &AtomicBool) -> Result<(), String> {
    let exe = tools().soffice.as_ref().ok_or("LibreOffice isn't installed")?;
    let filter = match (FORMATS[job.to].ext, FORMATS[job.from].media) {
        ("txt", _) => "txt:Text (encoded):UTF8",
        ("csv", _) => "csv:Text - txt - csv (StarCalc):44,34,76,1",
        ("docx", _) => "docx:MS Word 2007 XML",
        ("doc", _) => "doc:MS Word 97",
        ("xlsx", _) => "xlsx:Calc MS Excel 2007 XML",
        ("xls", _) => "xls:MS Excel 97",
        ("pptx", _) => "pptx:Impress MS PowerPoint 2007 XML",
        ("ppt", _) => "ppt:MS PowerPoint 97",
        (e, _) => e,
    };
    // A private profile: works while the user has LibreOffice open, and never touches theirs.
    let profile = dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("consertion").join("libreoffice");
    let out_dir = std::env::temp_dir().join(format!("consertion-lo-{}-{}", std::process::id(), tmp.file_name().map_or(String::new(), |n| n.to_string_lossy().into_owned())));
    std::fs::create_dir_all(&out_dir).map_err(|e| format!("Couldn't create a temporary folder: {e}"))?;
    let mut args = vec![
        os("--headless"),
        os("--norestore"),
        os("--nolockcheck"),
        os("--nologo"),
        os(format!("-env:UserInstallation={}", file_url(&profile))),
    ];
    if FORMATS[job.from].ext == "csv" {
        args.push(os("--infilter=CSV:44,34,76,1"));
    }
    args.extend([os("--convert-to"), os(filter), os("--outdir"), out_dir.clone().into(), job.input.clone().into()]);
    let result = run(exe, &args, cancel, None, "LibreOffice").and_then(|()| {
        // LibreOffice names the result after the input; take whatever it wrote.
        let made = std::fs::read_dir(&out_dir).ok().and_then(|mut d| d.find_map(|e| e.ok().map(|e| e.path()).filter(|p| p.is_file())));
        let made = made.ok_or("LibreOffice couldn't convert this file")?;
        std::fs::rename(&made, tmp).or_else(|_| std::fs::copy(&made, tmp).map(|_| ())).map_err(|e| format!("Couldn't save the file: {e}"))
    });
    let _ = std::fs::remove_dir_all(&out_dir);
    result
}

// ── Running a tool ───────────────────────────────────────────────────────────

/// Run to completion, killing it on cancel. ffmpeg's `-progress` output (stdout) becomes
/// progress events when the input's duration is known. Errors carry the tool's own last words.
fn run(exe: &Path, args: &[OsString], cancel: &AtomicBool, progress: Option<(f64, &Sender<Event>)>, name: &str) -> Result<(), String> {
    let mut child = Command::new(exe)
        .args(args)
        .stdin(Stdio::null())
        .stdout(if progress.is_some() { Stdio::piped() } else { Stdio::null() })
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Couldn't start {name}: {e}"))?;
    // Read both pipes on threads so a chatty tool can't fill a pipe and stall.
    let mut stderr = child.stderr.take();
    let err_reader = std::thread::spawn(move || {
        let mut s = String::new();
        if let Some(e) = stderr.as_mut() {
            let _ = e.take(1 << 20).read_to_string(&mut s);
        }
        s
    });
    let out_reader = match (child.stdout.take(), progress) {
        (Some(out), Some((total, tx))) => {
            let tx = tx.clone();
            Some(std::thread::spawn(move || {
                for line in BufReader::new(out).lines().map_while(Result::ok) {
                    if let Some(us) = line.strip_prefix("out_time_us=").or(line.strip_prefix("out_time_ms=")).and_then(|v| v.trim().parse::<f64>().ok()) {
                        let _ = tx.send(Event::Progress(((us / 1e6) / total).clamp(0.0, 1.0) as f32));
                    }
                }
            }))
        }
        _ => None,
    };
    let status = loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Cancelled".into());
        }
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => std::thread::sleep(Duration::from_millis(40)),
            Err(e) => return Err(format!("{name} stopped unexpectedly: {e}")),
        }
    };
    if let Some(r) = out_reader {
        let _ = r.join();
    }
    let err = err_reader.join().unwrap_or_default();
    if status.success() {
        return Ok(());
    }
    let last: Vec<&str> = err.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let tail = last[last.len().saturating_sub(2)..].join(" · ");
    let tail: String = tail.chars().take(240).collect();
    Err(if tail.is_empty() { format!("{name} failed ({status})") } else { format!("{name}: {tail}") })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_and_urls() {
        assert_eq!(ff_scale("720p", true).unwrap(), "scale=-2:'trunc(min(720,ih)/2)*2'");
        assert_eq!(ff_scale("50%", false).unwrap(), "scale='iw*0.5':'ih*0.5'");
        assert_eq!(ff_scale("Original", false), None);
        assert!(ff_scale("Original", true).is_some(), "videos keep even sizes");
        assert_eq!(file_url(Path::new("/tmp/a b")), "file:///tmp/a%20b");
        assert_eq!(file_url(Path::new(r"C:\x")), "file:///C:/x");
    }
}
