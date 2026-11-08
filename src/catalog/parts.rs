//! Markings printed on electronic parts: capacitor codes ("104"), SMD resistor codes ("4R7",
//! "1002") and resistor color bands. Values convert exactly; the base is the category's
//! SI unit (farad or ohm).

use crate::num::{self, int, R};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeFmt {
    /// Ceramic/film capacitor code in picofarads: "104" = 10 × 10⁴ pF.
    Cap,
    /// SMD resistor code: "103", "1002", "4R7".
    Smd,
    /// Resistor color bands: "brown black orange gold".
    Bands,
}

const COLORS: [(&str, &str); 10] = [
    ("black", "bk"), ("brown", "bn"), ("red", "rd"), ("orange", "og"), ("yellow", "ye"),
    ("green", "gn"), ("blue", "bu"), ("violet", "vt"), ("grey", "gy"), ("white", "wh"),
];

pub fn accepts(s: &str) -> bool {
    s.chars().count() <= 80 && s.chars().all(|c| c.is_ascii_alphanumeric() || " ,-/±%.".contains(c))
}

impl CodeFmt {
    pub fn placeholder(self) -> &'static str {
        match self {
            CodeFmt::Cap => "e.g. 104, 222J or 4R7",
            CodeFmt::Smd => "e.g. 103, 1002, 4R7 or R47",
            CodeFmt::Bands => "e.g. brown black orange gold",
        }
    }

    pub fn example(self) -> &'static str {
        match self {
            CodeFmt::Cap => "104",
            CodeFmt::Smd => "472",
            CodeFmt::Bands => "yellow violet red gold",
        }
    }

    pub fn parse(self, s: &str) -> Result<Option<R>, String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(None);
        }
        match self {
            CodeFmt::Cap => {
                // A trailing tolerance letter (J, K, M…) doesn't change the value.
                let body = s.strip_suffix(|c: char| c.is_ascii_alphabetic() && c != 'R' && c != 'r').unwrap_or(s);
                Ok(Some(digit_code(body, true)? * num::pow10(-12)))
            }
            CodeFmt::Smd => digit_code(s, false).map(Some),
            CodeFmt::Bands => bands(s).map(Some),
        }
    }

    pub fn format(self, q: &R) -> Result<String, String> {
        if q.is_negative() {
            return Err("A part value can't be negative".into());
        }
        match self {
            CodeFmt::Cap => {
                let pf = q / num::pow10(-12);
                let (m, e) = digits(&pf).ok_or("No exact code: a code holds 2 significant digits")?;
                if pf < int(10) {
                    return r_notation(&m, e, 2).ok_or_else(|| "Too small for a capacitor code".into());
                }
                let (mant, k) = pad(m, e, 2).ok_or("No exact code: a code holds 2 significant digits")?;
                if k > 7 {
                    return Err("Too large for a capacitor code".into());
                }
                Ok(format!("{mant}{k}"))
            }
            CodeFmt::Smd => {
                if q.is_zero() {
                    return Ok("000".into());
                }
                let (m, e) = digits(q).ok_or("No exact code: SMD codes hold up to 3 significant digits")?;
                if q < &int(10) {
                    return r_notation(&m, e, 3).ok_or_else(|| "Too small for an SMD code".into());
                }
                let (mant, k) = pad(m.clone(), e, 2).or_else(|| pad(m, e, 3)).ok_or("No exact code: SMD codes hold up to 3 significant digits")?;
                if k > 9 {
                    return Err("Too large for an SMD code".into());
                }
                Ok(format!("{mant}{k}"))
            }
            CodeFmt::Bands => {
                if q.is_zero() {
                    return Ok("black (zero-ohm link)".into());
                }
                let (m, e) = digits(q).ok_or("Needs more than 3 significant digits for color bands")?;
                let (mant, k, tol) = match pad(m.clone(), e, 2) {
                    Some((mant, k)) => (mant, k, "gold"),
                    None => pad(m, e, 3).map(|(mant, k)| (mant, k, "brown")).ok_or("Needs more than 3 significant digits for color bands")?,
                };
                let mult = match k {
                    -2 => "silver",
                    -1 => "gold",
                    0..=9 => COLORS[k as usize].0,
                    _ => return Err("Out of range for color bands".into()),
                };
                let mut out: Vec<&str> = mant.to_string().bytes().map(|d| COLORS[(d - b'0') as usize].0).collect();
                out.push(mult);
                out.push(tol);
                Ok(out.join(" "))
            }
        }
    }
}

/// `q` as m × 10^e with m an integer without trailing zeros (None if q isn't a short decimal).
fn digits(q: &R) -> Option<(BigInt, i64)> {
    if q.is_zero() {
        return Some((BigInt::zero(), 0));
    }
    let mut e = 0;
    let mut v = q.clone();
    while !v.is_integer() {
        v *= int(10);
        e -= 1;
        if e < -30 {
            return None;
        }
    }
    let mut m = v.to_integer();
    let ten = BigInt::from(10);
    while (&m % &ten).is_zero() {
        m /= &ten;
        e += 1;
    }
    Some((m, e))
}

/// Mantissa with exactly `width` digits and its exponent, if `m` fits in that many digits.
fn pad(m: BigInt, e: i64, width: usize) -> Option<(BigInt, i64)> {
    let len = m.to_string().len();
    (len <= width).then(|| (m * num::pow10(width as i64 - len as i64).to_integer(), e - (width as i64 - len as i64)))
}

/// Small values written with R as the decimal point: 4.7 → "4R7", 0.47 → "R47".
fn r_notation(m: &BigInt, e: i64, max: usize) -> Option<String> {
    let s = m.to_string();
    if s.len() > max {
        return None;
    }
    let (whole, frac) = if e >= 0 {
        (format!("{s}{}", "0".repeat(e as usize)), String::new())
    } else {
        let neg = (-e) as usize;
        if s.len() > neg {
            (s[..s.len() - neg].to_string(), s[s.len() - neg..].to_string())
        } else {
            (String::new(), format!("{}{s}", "0".repeat(neg - s.len())))
        }
    };
    Some(format!("{whole}R{}", if frac.is_empty() { "0" } else { &frac }))
}

/// "104" (two digits and a multiplier), "1002" (three and a multiplier), "4R7", "47" or "0".
/// With `cap`, 8 and 9 as the multiplier mean ×0.01 and ×0.1 (the capacitor convention).
fn digit_code(s: &str, cap: bool) -> Result<R, String> {
    let s = s.trim().to_ascii_uppercase();
    let bad = || format!("“{s}” isn't a part code");
    if s.contains('R') {
        let v = s.replacen('R', ".", 1);
        let v = if v.starts_with('.') { format!("0{v}") } else { v };
        if s == "R" || !v.chars().all(|c| c.is_ascii_digit() || c == '.') {
            return Err(bad());
        }
        let v = v.trim_end_matches('.');
        return num::parse_decimal(v).map_err(|_| bad());
    }
    if s.is_empty() || !s.chars().all(|c| c.is_ascii_digit()) {
        return Err(bad());
    }
    if s.chars().all(|c| c == '0') {
        return Ok(R::zero());
    }
    match s.len() {
        1 | 2 => Ok(int(s.parse().map_err(|_| bad())?)),
        3 | 4 => {
            let (m, k) = s.split_at(s.len() - 1);
            let k: i64 = k.parse().map_err(|_| bad())?;
            let k = if cap && k >= 8 { k - 10 } else { k };
            Ok(int(m.parse().map_err(|_| bad())?) * num::pow10(k))
        }
        _ => Err(bad()),
    }
}

fn bands(s: &str) -> Result<R, String> {
    let words: Vec<String> = s
        .split(|c: char| c.is_whitespace() || c == ',' || c == '-' || c == '/')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_ascii_lowercase().replace("gray", "grey").replace("purple", "violet"))
        .collect();
    let color = |w: &str| -> Result<i64, String> {
        match w {
            "gold" | "gd" => Ok(-1),
            "silver" | "sv" | "sr" => Ok(-2),
            _ => COLORS
                .iter()
                // Full name, two-letter code, or a short form like "brn" / "blk" (letters in order).
                .position(|&(n, a)| n == w || a == w || (w.len() >= 3 && n.starts_with(&w[..1]) && super::is_subsequence(n, w)))
                .map(|i| i as i64)
                .ok_or_else(|| format!("“{w}” isn't a band color")),
        }
    };
    let cols = words.iter().map(|w| color(w)).collect::<Result<Vec<_>, _>>()?;
    let n_digits = match cols.len() {
        1 if cols[0] == 0 => return Ok(R::zero()),
        3 | 4 => 2,
        5 | 6 => 3,
        _ => return Err("Use 3 to 6 bands, e.g. brown black orange gold".into()),
    };
    let mut mant = 0i64;
    for &d in &cols[..n_digits] {
        if d < 0 {
            return Err("Gold and silver are only multiplier or tolerance bands".into());
        }
        mant = mant * 10 + d;
    }
    Ok(int(mant) * num::pow10(cols[n_digits]))
}

/// Standard E-series values for one decade, in hundredths (E24 isn't a simple formula).
const E24: [i64; 24] = [100, 110, 120, 130, 150, 160, 180, 200, 220, 240, 270, 300, 330, 360, 390, 430, 470, 510, 560, 620, 680, 750, 820, 910];

/// The standard value nearest to `v` (by ratio) in E6, E12, E24, E48, E96 or E192.
pub fn nearest_e(v: &R, series: i64) -> Result<R, &'static str> {
    let f = v.to_f64().filter(|f| f.is_finite() && *f > 0.0).ok_or("Value must be above zero")?;
    let decade: Vec<i64> = match series {
        6 | 12 | 24 => E24.iter().step_by((24 / series) as usize).copied().collect(),
        48 | 96 | 192 => (0..series)
            .map(|i| {
                let x = (10f64.powf(i as f64 / series as f64) * 100.0).round() as i64;
                if series == 192 && x == 919 { 920 } else { x }
            })
            .collect(),
        _ => return Err("Series is 6, 12, 24, 48, 96 or 192"),
    };
    let exp = f.log10().floor() as i64;
    let mut best: Option<(f64, i64, i64)> = None;
    for e in [exp - 1, exp, exp + 1] {
        for &d in &decade {
            let c = d as f64 * 10f64.powi((e - 2) as i32);
            let dist = (c / f).ln().abs();
            if best.is_none_or(|b| dist < b.0) {
                best = Some((dist, d, e));
            }
        }
    }
    let (_, d, e) = best.ok_or("No value found")?;
    Ok(int(d) * num::pow10(e - 2))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(f: CodeFmt, s: &str) -> R {
        f.parse(s).unwrap().unwrap()
    }

    #[test]
    fn codes() {
        let pf = |v: i64| int(v) * num::pow10(-12);
        assert_eq!(p(CodeFmt::Cap, "104"), pf(100_000));
        assert_eq!(p(CodeFmt::Cap, "222J"), pf(2200));
        assert_eq!(p(CodeFmt::Cap, "4R7"), int(47) * num::pow10(-13));
        assert_eq!(p(CodeFmt::Cap, "479"), int(47) * num::pow10(-13));
        assert_eq!(p(CodeFmt::Cap, "47"), pf(47));
        assert_eq!(CodeFmt::Cap.format(&pf(100_000)).unwrap(), "104");
        assert_eq!(CodeFmt::Cap.format(&pf(10)).unwrap(), "100");
        assert_eq!(CodeFmt::Cap.format(&(int(47) * num::pow10(-13))).unwrap(), "4R7");
        assert!(CodeFmt::Cap.format(&pf(123)).is_err());

        assert_eq!(p(CodeFmt::Smd, "103"), int(10_000));
        assert_eq!(p(CodeFmt::Smd, "1002"), int(10_000));
        assert_eq!(p(CodeFmt::Smd, "R47"), int(47) / int(100));
        assert_eq!(p(CodeFmt::Smd, "000"), int(0));
        assert_eq!(CodeFmt::Smd.format(&int(10_000)).unwrap(), "103");
        assert_eq!(CodeFmt::Smd.format(&int(10_200)).unwrap(), "1022");
        assert_eq!(CodeFmt::Smd.format(&int(47)).unwrap(), "470");
        assert_eq!(CodeFmt::Smd.format(&(int(47) / int(10))).unwrap(), "4R7");
        assert_eq!(CodeFmt::Smd.format(&(int(47) / int(100))).unwrap(), "R47");
        assert_eq!(CodeFmt::Smd.format(&int(1)).unwrap(), "1R0");

        assert_eq!(p(CodeFmt::Bands, "brown black orange gold"), int(10_000));
        assert_eq!(p(CodeFmt::Bands, "Yellow, Violet, Gold, Gold"), int(47) / int(10));
        assert_eq!(p(CodeFmt::Bands, "brn blk blk red brn"), int(10_000));
        assert_eq!(p(CodeFmt::Bands, "gray red brown"), int(820));
        assert_eq!(CodeFmt::Bands.format(&int(10_000)).unwrap(), "brown black orange gold");
        assert_eq!(CodeFmt::Bands.format(&int(10_200)).unwrap(), "brown black red red brown");
        assert_eq!(CodeFmt::Bands.format(&(int(47) / int(10))).unwrap(), "yellow violet gold gold");
        assert!(bands("gold red red").is_err());
    }

    #[test]
    fn e_series() {
        let near = |v: &str, s| nearest_e(&num::parse_decimal(v).unwrap(), s).unwrap();
        assert_eq!(near("5000", 12), int(4700));
        assert_eq!(near("5000", 24), int(5100));
        assert_eq!(near("0.95", 12), int(1));
        assert_eq!(near("9900", 6), int(10_000));
        assert_eq!(near("4990", 96), int(4990));
    }
}
