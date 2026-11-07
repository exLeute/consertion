//! Dates (proleptic Gregorian calendar, UTC) and durations. Values are exact seconds:
//! a Unix timestamp for dates, a length of time for durations.

use crate::num::{self, int, R};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{Signed, ToPrimitive, Zero};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DateFmt {
    Iso,
    Rfc2822,
    /// "Saturday, 7 November 2026, 14:30:00 UTC" (output only)
    Readable,
    /// "2026-W45-6" (output only)
    IsoWeek,
    /// "2026-311"
    Ordinal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurFmt {
    /// "1:30:00", "2d 03:00:00"
    Clock,
    /// "2 d 3 h 15 min" (output only)
    Words,
}

/// About ±95 million years: keeps every intermediate comfortably inside i64.
const MAX_SECS: i64 = 3_000_000_000_000_000;

const MONTHS: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const WEEKDAYS: [&str; 7] = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"];

/// Days since 1970-01-01 (Howard Hinnant's algorithm; exact for any year).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let (m, d) = (m as i64, d as i64);
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719468
}

pub fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        2 if is_leap(y) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// ISO weeks in a year: 53 when it starts on a Thursday (or a Wednesday in leap years).
fn iso_weeks(y: i64) -> i64 {
    let p = |y: i64| (y + y.div_euclid(4) - y.div_euclid(100) + y.div_euclid(400)).rem_euclid(7);
    52 + i64::from(p(y) == 4 || p(y - 1) == 3)
}

pub fn now() -> R {
    let d = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    int(d.as_secs() as i64)
}

/// Today at 00:00 UTC.
pub fn today() -> R {
    let s = d_secs(&now());
    int(s.div_euclid(86400) * 86400)
}

fn d_secs(v: &R) -> i64 {
    v.floor().to_integer().to_i64().unwrap_or(0)
}

struct Parts {
    y: i64,
    mo: u32,
    d: u32,
    h: i64,
    mi: i64,
    s: i64,
    frac: R,
    /// Monday = 0
    weekday: usize,
    ordinal: i64,
}

fn parts(v: &R) -> Result<Parts, String> {
    let whole = v.floor();
    let secs = whole.to_integer().to_i64().filter(|s| s.abs() <= MAX_SECS).ok_or("Date out of range")?;
    let (days, sod) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    let (y, mo, d) = civil_from_days(days);
    Ok(Parts {
        y,
        mo,
        d,
        h: sod / 3600,
        mi: sod / 60 % 60,
        s: sod % 60,
        frac: v - whole,
        weekday: (days + 3).rem_euclid(7) as usize,
        ordinal: days - days_from_civil(y, 1, 1) + 1,
    })
}

/// ".25" for a quarter second; up to nanoseconds, "" for whole seconds.
fn frac_str(frac: &R) -> String {
    if frac.is_zero() {
        return String::new();
    }
    let ns = (frac * int(1_000_000_000)).round().to_integer();
    format!(".{ns:09}").trim_end_matches('0').to_string()
}

fn year_str(y: i64) -> String {
    if (0..=9999).contains(&y) { format!("{y:04}") } else { format!("{y:+05}") }
}

impl DateFmt {
    pub fn can_parse(self) -> bool {
        matches!(self, DateFmt::Iso | DateFmt::Rfc2822 | DateFmt::Ordinal)
    }

    pub fn accepts(self, s: &str) -> bool {
        s.len() <= 64
            && match self {
                DateFmt::Iso => s.chars().all(|c| c.is_ascii_digit() || "-+:.,TtZz ".contains(c)),
                DateFmt::Ordinal => s.chars().all(|c| c.is_ascii_digit() || c == '-' || c == '+'),
                _ => s.chars().all(|c| c.is_ascii_alphanumeric() || " ,:+-".contains(c)),
            }
    }

    pub fn placeholder(self) -> &'static str {
        match self {
            DateFmt::Iso => "YYYY-MM-DD hh:mm:ss",
            DateFmt::Rfc2822 => "Sat, 07 Nov 2026 14:30:00 +0000",
            DateFmt::Ordinal => "YYYY-DDD",
            _ => "output only",
        }
    }

    pub fn format(self, v: &R) -> Result<String, String> {
        let p = parts(v)?;
        let (day, month) = (WEEKDAYS[p.weekday], MONTHS[p.mo as usize - 1]);
        let time = format!("{:02}:{:02}:{:02}{}", p.h, p.mi, p.s, frac_str(&p.frac));
        Ok(match self {
            DateFmt::Iso => format!("{}-{:02}-{:02}T{time}Z", year_str(p.y), p.mo, p.d),
            DateFmt::Rfc2822 => format!("{}, {:02} {} {} {time} +0000", &day[..3], p.d, &month[..3], year_str(p.y)),
            DateFmt::Readable => format!("{day}, {} {month} {}, {time} UTC", p.d, p.y),
            DateFmt::IsoWeek => {
                let w = (p.ordinal - p.weekday as i64 - 1 + 10) / 7;
                let (wy, w) = match w {
                    0 => (p.y - 1, iso_weeks(p.y - 1)),
                    w if w > iso_weeks(p.y) => (p.y + 1, 1),
                    w => (p.y, w),
                };
                format!("{}-W{w:02}-{}", year_str(wy), p.weekday + 1)
            }
            DateFmt::Ordinal => format!("{}-{:03}", year_str(p.y), p.ordinal),
        })
    }

    /// Unix seconds; `None` while the input is still incomplete.
    pub fn parse(self, s: &str) -> Result<Option<R>, String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(None);
        }
        let mut c = Cur { s: s.as_bytes(), i: 0 };
        let r = match self {
            DateFmt::Iso => parse_iso(&mut c),
            DateFmt::Rfc2822 => parse_rfc(&mut c),
            DateFmt::Ordinal => (|| {
                let y = c.year()?;
                c.expect(b'-')?;
                let n = c.int(3, 3)?;
                let len = if is_leap(y) { 366 } else { 365 };
                if !(1..=len).contains(&n) {
                    return Err(Fail::Bad("Day of year must be 1 – 365 (366 in leap years)"));
                }
                Ok(int((days_from_civil(y, 1, 1) + n - 1) * 86400))
            })(),
            _ => return Err("This format is output only".into()),
        };
        match r {
            Ok(_) if c.i < c.s.len() => Err(format!("Unexpected \"{}\"", &s[c.i..])),
            Ok(v) => Ok(Some(v)),
            Err(Fail::Incomplete) => Ok(None),
            Err(Fail::Bad(m)) => Err(m.into()),
        }
    }
}

enum Fail {
    /// Ran out of input: keep typing.
    Incomplete,
    Bad(&'static str),
}

struct Cur<'a> {
    s: &'a [u8],
    i: usize,
}

impl Cur<'_> {
    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> bool {
        let hit = self.peek() == Some(c);
        self.i += usize::from(hit);
        hit
    }

    fn expect(&mut self, c: u8) -> Result<(), Fail> {
        match self.peek() {
            None => Err(Fail::Incomplete),
            Some(x) if x == c => {
                self.i += 1;
                Ok(())
            }
            _ => Err(Fail::Bad("Unexpected character")),
        }
    }

    fn spaces(&mut self) {
        while self.peek() == Some(b' ') {
            self.i += 1;
        }
    }

    /// Between `min` and `max` digits.
    fn int(&mut self, min: usize, max: usize) -> Result<i64, Fail> {
        let start = self.i;
        while self.i - start < max && self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.i += 1;
        }
        let n = self.i - start;
        if n < min {
            return Err(if self.i == self.s.len() { Fail::Incomplete } else { Fail::Bad("Expected a number") });
        }
        std::str::from_utf8(&self.s[start..self.i]).ok().and_then(|t| t.parse().ok()).ok_or(Fail::Bad("Number too large"))
    }

    fn year(&mut self) -> Result<i64, Fail> {
        let neg = self.eat(b'-');
        if !neg {
            self.eat(b'+');
        }
        let y = self.int(4, 9)?;
        Ok(if neg { -y } else { y })
    }

    fn word(&mut self) -> &str {
        let start = self.i;
        while self.peek().is_some_and(|c| c.is_ascii_alphabetic()) {
            self.i += 1;
        }
        std::str::from_utf8(&self.s[start..self.i]).unwrap_or("")
    }
}

fn date_secs(y: i64, mo: i64, d: i64) -> Result<i64, Fail> {
    if !(1..=12).contains(&mo) {
        return Err(Fail::Bad("Month must be 1 – 12"));
    }
    if d < 1 || d > days_in_month(y, mo as u32) as i64 {
        return Err(Fail::Bad("No such day in that month"));
    }
    Ok(days_from_civil(y, mo as u32, d as u32) * 86400)
}

/// "hh:mm[:ss[.fff]]" → seconds into the day.
fn time_of_day(c: &mut Cur) -> Result<R, Fail> {
    let h = c.int(2, 2)?;
    c.expect(b':')?;
    let mi = c.int(2, 2)?;
    let mut secs = int(0);
    if c.eat(b':') {
        let s = c.int(2, 2)?;
        secs = int(s);
        if c.eat(b'.') || c.eat(b',') {
            let start = c.i;
            c.int(1, 9)?;
            let digits = std::str::from_utf8(&c.s[start..c.i]).unwrap_or("0");
            secs += num::parse_decimal(&format!("0.{digits}")).map_err(|_| Fail::Bad("Bad fraction"))?;
        }
        if s > 60 {
            return Err(Fail::Bad("Seconds must be 0 – 59"));
        }
    }
    if h > 24 || mi > 59 || (h == 24 && (mi > 0 || !secs.is_zero())) {
        return Err(Fail::Bad("Time must be 00:00 – 24:00"));
    }
    Ok(int(h * 3600 + mi * 60) + secs)
}

/// "Z", "+02:00", "-0530", "UTC", "GMT" or nothing (UTC) → seconds east of UTC.
fn zone(c: &mut Cur) -> Result<i64, Fail> {
    c.spaces();
    match c.peek() {
        None => Ok(0),
        Some(b'Z' | b'z') => {
            c.i += 1;
            Ok(0)
        }
        Some(sign @ (b'+' | b'-')) => {
            c.i += 1;
            let h = c.int(2, 2)?;
            c.eat(b':');
            let m = c.int(2, 2)?;
            if h > 23 || m > 59 {
                return Err(Fail::Bad("Bad time zone offset"));
            }
            Ok(if sign == b'-' { -1 } else { 1 } * (h * 3600 + m * 60))
        }
        Some(_) => {
            let w = c.word().to_ascii_uppercase();
            match w.as_str() {
                "UTC" | "GMT" | "UT" => Ok(0),
                w if c.peek().is_none() && ("UTC".starts_with(w) || "GMT".starts_with(w)) => Err(Fail::Incomplete),
                _ => Err(Fail::Bad("Unknown time zone (use Z, UTC or ±hh:mm)")),
            }
        }
    }
}

fn parse_iso(c: &mut Cur) -> Result<R, Fail> {
    let y = c.year()?;
    c.expect(b'-')?;
    let mo = c.int(2, 2)?;
    c.expect(b'-')?;
    let d = c.int(2, 2)?;
    let day = date_secs(y, mo, d)?;
    let tod = match c.peek() {
        Some(b'T' | b't' | b' ') => {
            c.i += 1;
            c.spaces();
            if c.peek().is_none() {
                return Err(Fail::Incomplete);
            }
            time_of_day(c)?
        }
        _ => int(0),
    };
    let off = zone(c)?;
    Ok(int(day - off) + tod)
}

/// "Sat, 07 Nov 2026 14:30:00 +0000" (weekday optional).
fn parse_rfc(c: &mut Cur) -> Result<R, Fail> {
    c.spaces();
    if c.peek().is_some_and(|x| x.is_ascii_alphabetic()) {
        let w = c.word().to_ascii_lowercase();
        if !WEEKDAYS.iter().any(|d| d[..3].eq_ignore_ascii_case(&w)) {
            return Err(if c.peek().is_none() { Fail::Incomplete } else { Fail::Bad("Unknown weekday") });
        }
        c.eat(b',');
        c.spaces();
    }
    let d = c.int(1, 2)?;
    c.spaces();
    let m = c.word().to_ascii_lowercase();
    if c.peek().is_none() {
        return Err(Fail::Incomplete);
    }
    let mo = MONTHS.iter().position(|x| x[..3].eq_ignore_ascii_case(&m)).ok_or(Fail::Bad("Unknown month"))? as i64 + 1;
    c.spaces();
    let y = c.int(4, 9)?;
    let day = date_secs(y, mo, d)?;
    c.spaces();
    let tod = if c.peek().is_some() { time_of_day(c)? } else { int(0) };
    let off = zone(c)?;
    Ok(int(day - off) + tod)
}

impl DurFmt {
    pub fn accepts(self, s: &str) -> bool {
        s.len() <= 64 && s.chars().all(|c| c.is_ascii_digit() || " :.,dD-".contains(c))
    }

    pub fn format(self, v: &R) -> String {
        let sign = if v.is_negative() { "-" } else { "" };
        let v = v.abs();
        let whole = v.floor().to_integer();
        let frac = frac_str(&(&v - v.floor()));
        let (days, rest) = whole.div_rem(&BigInt::from(86400));
        let rest = rest.to_i64().unwrap_or(0);
        let (h, m, s) = (rest / 3600, rest / 60 % 60, rest % 60);
        match self {
            DurFmt::Clock if days.is_zero() => format!("{sign}{h}:{m:02}:{s:02}{frac}"),
            DurFmt::Clock => format!("{sign}{days}d {h:02}:{m:02}:{s:02}{frac}"),
            DurFmt::Words => {
                let mut out: Vec<String> = Vec::new();
                if !days.is_zero() {
                    out.push(format!("{days} d"));
                }
                for (n, u) in [(h, "h"), (m, "min")] {
                    if n > 0 {
                        out.push(format!("{n} {u}"));
                    }
                }
                if s > 0 || !frac.is_empty() || out.is_empty() {
                    out.push(format!("{s}{frac} s"));
                }
                format!("{sign}{}", out.join(" "))
            }
        }
    }

    /// "[Nd] h[:mm[:ss[.f]]]" → seconds. The last field may have a fraction.
    pub fn parse(self, s: &str) -> Result<Option<R>, String> {
        let s = s.trim();
        let (neg, s) = s.strip_prefix('-').map_or((false, s), |r| (true, r.trim_start()));
        if s.is_empty() {
            return Ok(None);
        }
        let (days, clock) = match s.split_once(['d', 'D']) {
            Some((d, rest)) => (d.trim(), rest.trim()),
            None => ("", s),
        };
        let field = |t: &str| -> Result<Option<R>, String> {
            match num::eval_input(&t.replace(',', ".")) {
                Ok(v) if !v.is_negative() => Ok(Some(v)),
                Ok(_) => Err("Use a leading \"-\" for negative durations".into()),
                Err(num::EvalError::Incomplete) => Ok(None),
                Err(num::EvalError::Invalid(m)) => Err(m),
            }
        };
        let mut total = match days {
            "" => int(0),
            d => match field(d)? {
                Some(v) => v * int(86400),
                None => return Ok(None),
            },
        };
        if !clock.is_empty() {
            let f: Vec<&str> = clock.split(':').collect();
            if f.len() > 3 {
                return Err("Use h:mm:ss".into());
            }
            for (i, t) in f.iter().enumerate() {
                let Some(v) = field(t)? else { return Ok(None) };
                if i + 1 < f.len() && !v.is_integer() {
                    return Err("Only the last field can have a fraction".into());
                }
                if i > 0 && v >= int(60) {
                    return Err("Minutes and seconds must be below 60".into());
                }
                total += v * int([3600, 60, 1][i]);
            }
        }
        Ok(Some(if neg { -total } else { total }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso(s: &str) -> R {
        DateFmt::Iso.parse(s).unwrap().unwrap()
    }

    #[test]
    fn calendar_roundtrip() {
        for z in [-800_000, -719_468, -1, 0, 1, 10_957, 20_399, 2_932_896] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(days_from_civil(2000, 3, 1), 11_017);
    }

    #[test]
    fn dates() {
        assert_eq!(iso("1970-01-01"), int(0));
        assert_eq!(iso("2026-11-07T14:30:00Z"), int(1_794_061_800));
        assert_eq!(iso("2026-11-07 16:30+02:00"), int(1_794_061_800));
        assert_eq!(DateFmt::Rfc2822.parse("Sat, 07 Nov 2026 14:30:00 +0000").unwrap(), Some(int(1_794_061_800)));
        assert_eq!(DateFmt::Iso.format(&int(1_794_061_800)).unwrap(), "2026-11-07T14:30:00Z");
        assert_eq!(DateFmt::Readable.format(&int(1_794_061_800)).unwrap(), "Saturday, 7 November 2026, 14:30:00 UTC");
        assert_eq!(DateFmt::IsoWeek.format(&iso("2026-11-07")).unwrap(), "2026-W45-6");
        assert_eq!(DateFmt::IsoWeek.format(&iso("2021-01-03")).unwrap(), "2020-W53-7");
        assert_eq!(DateFmt::Ordinal.format(&iso("2024-12-31")).unwrap(), "2024-366");
        assert_eq!(DateFmt::Iso.format(&(int(-1) / int(4))).unwrap(), "1969-12-31T23:59:59.75Z");
        assert_eq!(DateFmt::Iso.parse("2026-11"), Ok(None), "incomplete");
        assert!(DateFmt::Iso.parse("2026-02-30").is_err());
        assert!(DateFmt::Iso.parse("2026-11-07X").is_err());
    }

    #[test]
    fn durations() {
        let p = |s| DurFmt::Clock.parse(s).unwrap().unwrap();
        assert_eq!(p("1:30"), int(5400));
        assert_eq!(p("2d 1:00:00.5"), int(2 * 86400 + 3600) + int(1) / int(2));
        assert_eq!(DurFmt::Clock.format(&int(5400)), "1:30:00");
        assert_eq!(DurFmt::Clock.format(&int(90061)), "1d 01:01:01");
        assert_eq!(DurFmt::Words.format(&int(90061)), "1 d 1 h 1 min 1 s");
        assert!(DurFmt::Clock.parse("1:75").is_err());
    }
}
