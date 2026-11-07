//! Music: pitch (note names, MIDI numbers, Hz), tempo and note lengths, intervals, and audio
//! samples / video frames. Note names and intervals are computed in floating point.

use super::scale::Scale;
use super::{Builder, Kind};
use crate::num::{self, R};
use num_traits::ToPrimitive;

const NOTES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

pub fn register(b: &mut Builder) {
    b.category("pitch", "Musical Pitch", false);
    b.add("note name", "note", Kind::Note, "pitch note a4 c4 middle c sharp flat tuning", true);
    b.linear(&[("hertz", "Hz", "1", "frequency pitch tuning"), ("kilohertz", "kHz", "1000", "frequency")]);
    b.scale("MIDI note number", "MIDI", Scale::Midi, "midi note number daw piano roll");
    b.reciprocal("period (ms)", "ms period", "1000", "cycle length wave period");
    b.reciprocal("wavelength in air (20 °C)", "λ m", "343", "sound wavelength speed of sound acoustics");

    b.category("tempo", "Tempo & Note Lengths", false);
    b.linear(&[
        ("beats per minute", "BPM", "1", "tempo bpm beat"),
        ("beats per second", "beats/s", "60", "tempo hz lfo rate"),
    ]);
    // Note lengths in ms at a tempo where a beat is a quarter note.
    for (name, sym, quarters) in [
        ("whole note", "1/1", "4"),
        ("half note", "1/2", "2"),
        ("dotted half note", "1/2.", "3"),
        ("half-note triplet", "1/2T", "4/3"),
        ("quarter note", "1/4", "1"),
        ("dotted quarter note", "1/4.", "3/2"),
        ("quarter-note triplet", "1/4T", "2/3"),
        ("eighth note", "1/8", "1/2"),
        ("dotted eighth note", "1/8.", "3/4"),
        ("eighth-note triplet", "1/8T", "1/3"),
        ("sixteenth note", "1/16", "1/4"),
        ("dotted sixteenth note", "1/16.", "3/8"),
        ("sixteenth-note triplet", "1/16T", "1/6"),
        ("thirty-second note", "1/32", "1/8"),
    ] {
        b.reciprocal(&format!("{name} (ms)"), &format!("{sym} ms"), &format!("60000*{quarters}"), "delay time reverb note length tempo sync");
    }
    b.reciprocal("bar of 4/4 (seconds)", "bar 4/4", "240", "measure length");
    b.reciprocal("bar of 3/4 (seconds)", "bar 3/4", "180", "measure length waltz");
    b.reciprocal("quarter note (samples at 44.1 kHz)", "1/4 smp44.1", "60*44100", "samples delay");
    b.reciprocal("quarter note (samples at 48 kHz)", "1/4 smp48", "60*48000", "samples delay");

    b.category("interval", "Musical Intervals", false);
    b.add("interval name", "interval", Kind::Interval, "fifth third octave semitone tone harmony chord", true);
    b.linear(&[
        ("cent", "¢", "1", "cents tuning detune"),
        ("semitone", "st", "100", "half step semitones transpose"),
        ("whole tone", "tone", "200", "whole step"),
        ("octave", "oct", "1200", "octaves"),
    ]);
    b.scale("frequency ratio", "ratio", Scale::FreqRatio, "just intonation 3/2 5/4 pitch ratio");

    b.category("frames", "Audio Samples & Video Frames", false);
    b.linear(&[("second", "s", "1", "time"), ("millisecond", "ms", "1/1000", "time latency")]);
    for (rate, label) in [("22050", "22.05 kHz"), ("44100", "44.1 kHz"), ("48000", "48 kHz"), ("88200", "88.2 kHz"), ("96000", "96 kHz"), ("192000", "192 kHz")] {
        b.unit(&format!("samples at {label}"), &format!("smp {label}"), Kind::Linear, super::expr(&format!("1/{rate}")), R::from_integer(0.into()), "audio sample rate buffer latency", true);
    }
    for (fps, label) in [("24000/1001", "23.976"), ("24", "24"), ("25", "25"), ("30000/1001", "29.97"), ("30", "30"), ("50", "50"), ("60", "60")] {
        b.unit(&format!("frames at {label} fps"), &format!("fr {label}"), Kind::Linear, super::expr(&format!("1/({fps})")), R::from_integer(0.into()), "video frame rate film", true);
    }
}

/// MIDI note number (fractional) of a frequency.
pub fn midi_of(hz: f64) -> f64 {
    69.0 + 12.0 * (hz / 440.0).log2()
}

pub fn hz_of(midi: f64) -> f64 {
    440.0 * 2f64.powf((midi - 69.0) / 12.0)
}

pub fn accepts_note(s: &str) -> bool {
    s.len() <= 20 && s.chars().all(|c| c.is_ascii_alphanumeric() || "#♯♭-+−¢. ".contains(c))
}

/// "A4", "C#5", "Bb3", "c", "F#2 +15¢". Octave 4 when left out. Hz.
pub fn parse_note(s: &str) -> Result<Option<R>, String> {
    let s = s.trim();
    let mut chars = s.chars().peekable();
    let Some(letter) = chars.next() else { return Ok(None) };
    let base = match letter.to_ascii_uppercase() {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return Err("Notes are A–G, e.g. A4, C#5 or Bb3".into()),
    };
    let mut semis = base as f64;
    while let Some(&c) = chars.peek() {
        match c {
            '#' | '♯' => semis += 1.0,
            'b' | '♭' => semis -= 1.0,
            _ => break,
        }
        chars.next();
    }
    let rest: String = chars.collect::<String>().replace('−', "-");
    let rest = rest.trim();
    // Octave, then optional cents ("+15", "-3¢").
    let split = rest.char_indices().skip(1).find(|(_, c)| *c == '+' || *c == '-' || *c == ' ').map_or(rest.len(), |(i, _)| i);
    let (oct, cents) = rest.split_at(split);
    let octave: f64 = match oct.trim() {
        "" => 4.0,
        "-" => return Ok(None),
        o => o.parse::<i32>().map_err(|_| "Octave is a whole number, e.g. A4 or C-1".to_string())? as f64,
    };
    let cents = cents.trim().trim_end_matches(['¢', 'c']).replace(' ', "");
    let cents: f64 = match cents.as_str() {
        "" => 0.0,
        "+" | "-" => return Ok(None),
        c => c.parse().map_err(|_| "Cents look like +15 or -3".to_string())?,
    };
    if !(-2.0..=10.0).contains(&octave) {
        return Err("Octave must be between -2 and 10".into());
    }
    let midi = (octave + 1.0) * 12.0 + semis + cents / 100.0;
    num::from_f64_rounded(hz_of(midi)).map(Some).ok_or("Note out of range".into())
}

/// "A4", "C#5 +12¢".
pub fn format_note(hz: &R) -> Result<String, String> {
    let hz = hz.to_f64().filter(|h| *h > 0.0 && h.is_finite()).ok_or("A note needs a frequency above zero")?;
    let midi = midi_of(hz);
    if !(-24.0..=150.0).contains(&midi) {
        return Err("Outside the range of note names".into());
    }
    let n = midi.round();
    let cents = ((midi - n) * 100.0).round();
    let name = format!("{}{}", NOTES[(n as i64).rem_euclid(12) as usize], (n as i64).div_euclid(12) - 1);
    Ok(match cents as i64 {
        0 => name,
        c if c > 0 => format!("{name} +{c}¢"),
        c => format!("{name} −{}¢", -c),
    })
}

/// (semitones, name, short names accepted when typing; matched case-sensitively).
const INTERVALS: [(i32, &str, &[&str]); 25] = [
    (0, "unison", &["P1"]),
    (1, "minor second", &["m2", "semitone", "half step"]),
    (2, "major second", &["M2", "whole tone", "whole step"]),
    (3, "minor third", &["m3"]),
    (4, "major third", &["M3"]),
    (5, "perfect fourth", &["P4", "fourth"]),
    (6, "tritone", &["TT", "A4", "d5", "augmented fourth", "diminished fifth"]),
    (7, "perfect fifth", &["P5", "fifth"]),
    (8, "minor sixth", &["m6"]),
    (9, "major sixth", &["M6"]),
    (10, "minor seventh", &["m7"]),
    (11, "major seventh", &["M7"]),
    (12, "octave", &["P8"]),
    (13, "minor ninth", &["m9"]),
    (14, "major ninth", &["M9", "ninth"]),
    (15, "minor tenth", &["m10"]),
    (16, "major tenth", &["M10"]),
    (17, "perfect eleventh", &["P11", "eleventh"]),
    (18, "augmented eleventh", &["A11"]),
    (19, "perfect twelfth", &["P12"]),
    (20, "minor thirteenth", &["m13"]),
    (21, "major thirteenth", &["M13", "thirteenth"]),
    (22, "minor fourteenth", &["m14"]),
    (23, "major fourteenth", &["M14"]),
    (24, "double octave", &["P15", "two octaves"]),
];

pub fn accepts_interval(s: &str) -> bool {
    s.len() <= 40 && s.chars().all(|c| c.is_ascii_alphanumeric() || " -+−¢".contains(c))
}

/// "P5", "minor third", "-octave", "M3 +14¢". Cents.
pub fn parse_interval(s: &str) -> Result<Option<R>, String> {
    let s = s.trim().replace('−', "-");
    let (sign, s) = match s.strip_prefix('-') {
        Some(r) => (-1.0, r.trim()),
        None => (1.0, s.trim()),
    };
    if s.is_empty() {
        return Ok(None);
    }
    // Optional "+14¢" at the end.
    let (name, cents) = match s.rfind(['+', '-']) {
        Some(i) if i > 0 => (s[..i].trim(), s[i..].trim_end_matches(['¢', 'c']).trim()),
        _ => (s, ""),
    };
    let cents: f64 = match cents {
        "" => 0.0,
        "+" | "-" => return Ok(None),
        c => c.replace(' ', "").parse().map_err(|_| "Cents look like +14 or -2".to_string())?,
    };
    let lower = name.to_lowercase();
    let found = INTERVALS.iter().find(|(_, full, short)| *full == lower || short.iter().any(|a| *a == name || (a.len() > 3 && a.to_lowercase() == lower)));
    let Some(&(semis, _, _)) = found else {
        return Err("Try P5, m3, M6, tritone, octave or a full name like minor third".into());
    };
    Ok(num::from_f64_rounded(sign * (semis as f64 * 100.0 + cents)))
}

/// "perfect fifth", "major third +14¢", "2 octaves + perfect fourth", "down a minor third".
pub fn format_interval(cents: &R) -> Result<String, String> {
    let c = cents.to_f64().filter(|c| c.is_finite() && c.abs() < 1e6).ok_or("Interval out of range")?;
    let semis = (c.abs() / 100.0).round() as i64;
    let off = ((c.abs() - semis as f64 * 100.0) * c.signum()).round() as i64;
    let name = if semis <= 24 {
        INTERVALS[semis as usize].1.to_string()
    } else {
        let (oct, rest) = (semis / 12, semis % 12);
        if rest == 0 { format!("{oct} octaves") } else { format!("{oct} octaves + {}", INTERVALS[rest as usize].1) }
    };
    let name = if c < 0.0 && semis > 0 { format!("down a {name}") } else { name };
    Ok(match off {
        0 => name,
        o if o > 0 => format!("{name} +{o}¢"),
        o => format!("{name} −{}¢", -o),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::Notation;

    fn hz(s: &str) -> String {
        num::format(&parse_note(s).unwrap().unwrap(), 6, Notation::Auto).0
    }

    #[test]
    fn notes_and_intervals() {
        assert_eq!(hz("A4"), "440");
        assert_eq!(hz("a"), "440");
        assert_eq!(hz("C4"), "261.626");
        assert_eq!(hz("Bb3"), "233.082");
        assert_eq!(hz("C-1"), "8.1758");
        assert_eq!(hz("A4 +100¢"), hz("A#4"));
        assert_eq!(format_note(&parse_note("C#5").unwrap().unwrap()).unwrap(), "C#5");
        assert_eq!(format_note(&R::from_integer(445.into())).unwrap(), "A4 +20¢");
        assert_eq!(format_note(&R::from_integer(435.into())).unwrap(), "A4 −20¢");
        assert_eq!(parse_note("A"), parse_note("A4"));
        assert_eq!(parse_note("A4 +"), Ok(None));
        assert!(parse_note("H2").is_err());

        let cents = |s: &str| parse_interval(s).unwrap().unwrap();
        assert_eq!(cents("P5"), R::from_integer(700.into()));
        assert_eq!(cents("minor third"), R::from_integer(300.into()));
        assert_eq!(cents("m3"), cents("Minor Third"));
        assert_ne!(cents("m3"), cents("M3"));
        assert_eq!(cents("-octave"), R::from_integer((-1200).into()));
        assert_eq!(format_interval(&R::from_integer(702.into())).unwrap(), "perfect fifth +2¢");
        assert_eq!(format_interval(&R::from_integer(1900.into())).unwrap(), "perfect twelfth");
        assert_eq!(format_interval(&R::from_integer(2900.into())).unwrap(), "2 octaves + perfect fourth");
        assert_eq!(format_interval(&R::from_integer((-300).into())).unwrap(), "down a minor third");
        assert!(parse_interval("P6").is_err());
    }
}
