//! Number systems: bases 2–36, Roman numerals, English words. Integers of any size.

use super::{Builder, Kind};
use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

const BASE_NAMES: &[(u32, &str, &str)] = &[
    (2, "Binary", "bin"),
    (3, "Ternary", "ter"),
    (4, "Quaternary", "quat"),
    (5, "Quinary", "quin"),
    (6, "Senary", "sen"),
    (7, "Septenary", "sep"),
    (8, "Octal", "oct"),
    (9, "Nonary", "non"),
    (10, "Decimal", "dec"),
    (11, "Undecimal", "undec"),
    (12, "Duodecimal", "doz"),
    (16, "Hexadecimal", "hex"),
    (20, "Vigesimal", "vig"),
    (32, "Base32 digits", "b32"),
    (36, "Hexatrigesimal", "b36"),
];

/// Longest accepted digit string (≈ 3300 decimal digits in binary).
const MAX_DIGITS: usize = 11_000;

pub fn register(b: &mut Builder) {
    b.category("numbers", "Number Systems", false);
    for r in 2..=36u32 {
        let (name, sym, common) = match BASE_NAMES.iter().find(|n| n.0 == r) {
            Some(&(_, n, s)) => (format!("{n} (base {r})"), s.to_string(), matches!(r, 2 | 8 | 10 | 16)),
            None => (format!("Base {r}"), format!("b{r}"), false),
        };
        b.add(&name, &sym, Kind::Radix(r), &format!("radix base{r} integer"), common);
    }
    b.add("Roman numerals", "XIV", Kind::Roman, "roman", true);
    b.add("English words", "words", Kind::Words, "spell out number to words text", true);
}

pub fn accepts_radix(s: &str, radix: u32) -> bool {
    let digits = s.strip_prefix('-').unwrap_or(s);
    digits.len() <= MAX_DIGITS && digits.chars().all(|c| c.is_digit(radix))
}

pub fn parse_radix(s: &str, radix: u32) -> Result<Option<BigInt>, String> {
    if s.is_empty() || s == "-" {
        return Ok(None);
    }
    BigInt::parse_bytes(s.as_bytes(), radix).map(Some).ok_or("Invalid digits for this base".into())
}

pub fn to_radix(n: &BigInt, radix: u32) -> String {
    n.to_str_radix(radix).to_uppercase()
}

const ROMAN: [(i64, &str); 13] = [
    (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
    (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
];

pub fn to_roman(n: &BigInt) -> Result<String, String> {
    let mut n = n.to_i64().filter(|n| (1..=3999).contains(n)).ok_or("Roman numerals cover 1 – 3999")?;
    let mut s = String::new();
    for (v, r) in ROMAN {
        while n >= v {
            s.push_str(r);
            n -= v;
        }
    }
    Ok(s)
}

pub fn parse_roman(s: &str) -> Result<BigInt, String> {
    let s = s.to_uppercase();
    let val = |c| match c {
        'I' => 1, 'V' => 5, 'X' => 10, 'L' => 50, 'C' => 100, 'D' => 500, 'M' => 1000, _ => 0,
    };
    let chars: Vec<i64> = s.chars().map(val).collect();
    let mut total = 0;
    for (i, &v) in chars.iter().enumerate() {
        if chars.get(i + 1).is_some_and(|&next| next > v) { total -= v } else { total += v }
    }
    // Reject non-canonical forms like "IIII" or "IC" by round-tripping.
    let n = BigInt::from(total);
    match to_roman(&n) {
        Ok(r) if r == s => Ok(n),
        _ => Err("Not a valid Roman numeral".into()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frac {
    /// "11/4"
    Fraction,
    /// "2 3/4"
    Mixed,
}

/// Exact value as a reduced fraction; whole numbers stay plain.
pub fn format_fraction(q: &crate::num::R, f: Frac) -> String {
    if q.is_integer() {
        return q.numer().to_string();
    }
    let (n, d) = (q.numer(), q.denom());
    match f {
        Frac::Fraction => format!("{n}/{d}"),
        Frac::Mixed => {
            let (whole, rest) = (n.abs() / d, n.abs() % d);
            let sign = if n.is_negative() { "-" } else { "" };
            if whole.is_zero() { format!("{sign}{rest}/{d}") } else { format!("{sign}{whole} {rest}/{d}") }
        }
    }
}

/// "2 3/4" → "(2+3/4)", "-2 3/4" → "-(2+3/4)", so a mixed number can be evaluated.
pub fn mixed_to_expr(s: &str) -> String {
    let t = s.trim();
    let (neg, body) = t.strip_prefix('-').map_or((false, t), |r| (true, r.trim_start()));
    match body.split_once(' ') {
        Some((w, f)) if !w.is_empty() && w.chars().all(|c| c.is_ascii_digit()) && f.trim().contains('/') => {
            let e = format!("({w}+{})", f.trim());
            if neg { format!("-{e}") } else { e }
        }
        _ => s.to_string(),
    }
}

const ONES: [&str; 20] = [
    "zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine", "ten",
    "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen", "seventeen", "eighteen", "nineteen",
];
const TENS: [&str; 10] = ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"];
const SCALES: [&str; 22] = [
    "", "thousand", "million", "billion", "trillion", "quadrillion", "quintillion", "sextillion",
    "septillion", "octillion", "nonillion", "decillion", "undecillion", "duodecillion", "tredecillion",
    "quattuordecillion", "quindecillion", "sexdecillion", "septendecillion", "octodecillion",
    "novemdecillion", "vigintillion",
];

pub fn to_words(n: &BigInt) -> Result<String, String> {
    if n.is_zero() {
        return Ok(ONES[0].into());
    }
    let mut groups = Vec::new();
    let mut u = n.abs();
    let thousand = BigInt::from(1000);
    while !u.is_zero() {
        groups.push((&u % &thousand).to_usize().unwrap());
        u /= &thousand;
    }
    if groups.len() > SCALES.len() {
        return Err("Too large to spell out (limit: vigintillions, 10^66)".into());
    }
    let mut parts = Vec::new();
    for (i, &g) in groups.iter().enumerate().rev() {
        if g == 0 {
            continue;
        }
        let mut p = Vec::new();
        if g >= 100 {
            p.push(format!("{} hundred", ONES[g / 100]));
        }
        let r = g % 100;
        if r >= 20 {
            p.push(if r % 10 == 0 { TENS[r / 10].to_string() } else { format!("{}-{}", TENS[r / 10], ONES[r % 10]) });
        } else if r > 0 {
            p.push(ONES[r].to_string());
        }
        if i > 0 {
            p.push(SCALES[i].to_string());
        }
        parts.push(p.join(" "));
    }
    let words = parts.join(" ");
    Ok(if n.is_negative() { format!("minus {words}") } else { words })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn big(s: &str) -> BigInt {
        s.parse().unwrap()
    }

    #[test]
    fn radix_roundtrip() {
        assert_eq!(to_radix(&big("255"), 16), "FF");
        assert_eq!(to_radix(&big("-10"), 2), "-1010");
        assert_eq!(parse_radix("-ff", 16), Ok(Some(big("-255"))));
        assert!(accepts_radix("102", 3) && !accepts_radix("12", 2) && !accepts_radix("1-", 2));
        let huge = "9".repeat(200);
        assert_eq!(to_radix(&parse_radix(&to_radix(&big(&huge), 36), 36).unwrap().unwrap(), 10), huge);
    }

    #[test]
    fn roman() {
        assert_eq!(to_roman(&big("2026")).unwrap(), "MMXXVI");
        assert_eq!(parse_roman("mcmxciv"), Ok(big("1994")));
        assert!(parse_roman("IIII").is_err() && parse_roman("IC").is_err());
        assert!(to_roman(&big("0")).is_err());
    }

    #[test]
    fn fractions() {
        let q = crate::num::eval_input("-11/4").unwrap();
        assert_eq!(format_fraction(&q, Frac::Fraction), "-11/4");
        assert_eq!(format_fraction(&q, Frac::Mixed), "-2 3/4");
        assert_eq!(format_fraction(&crate::num::eval_input("0.375").unwrap(), Frac::Mixed), "3/8");
        assert_eq!(crate::num::eval_input(&mixed_to_expr("-2 3/4")).unwrap(), q);
    }

    #[test]
    fn words() {
        assert_eq!(to_words(&big("1234567")).unwrap(), "one million two hundred thirty-four thousand five hundred sixty-seven");
        assert_eq!(to_words(&big("-40")).unwrap(), "minus forty");
        assert!(to_words(&big(&"9".repeat(66))).is_ok());
        assert!(to_words(&big(&"9".repeat(67))).is_err());
    }
}
