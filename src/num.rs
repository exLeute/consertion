//! Exact arithmetic: arbitrary-size rationals, an expression evaluator, and decimal formatting.
//! Conversions never touch floating point, so defined units convert exactly (1 mi = 63360 in).

use num_bigint::BigInt;
use num_rational::BigRational;
use num_integer::Integer;
use num_traits::{One, Pow, Signed, ToPrimitive, Zero};
use serde::{Deserialize, Serialize};

pub type R = BigRational;

/// Largest decimal exponent accepted or produced. Keeps everything instant.
pub const MAX_EXP: i64 = 10_000;

#[derive(Debug, PartialEq)]
pub enum EvalError {
    /// Input ends mid-expression ("2*", "1e"): show nothing yet.
    Incomplete,
    Invalid(String),
}

pub fn int(n: i64) -> R {
    R::from_integer(n.into())
}

fn ten(e: i64) -> BigInt {
    BigInt::from(10).pow(e.unsigned_abs() as u32)
}

pub fn pow10(e: i64) -> R {
    if e >= 0 { R::from_integer(ten(e)) } else { R::new_raw(BigInt::one(), ten(e)) }
}

// num-rational reduces every result with num-bigint's binary gcd, which takes one full-width
// shift per bit and crawls on 5000-digit values. Hot paths use these Euclid-reduced ops instead.

pub fn gcd(a: &BigInt, b: &BigInt) -> BigInt {
    let (mut a, mut b) = (a.abs(), b.abs());
    while !b.is_zero() {
        let r = &a % &b;
        a = std::mem::replace(&mut b, r);
    }
    a
}

/// n / d in lowest terms (d ≠ 0).
pub fn ratio(n: BigInt, d: BigInt) -> R {
    let g = gcd(&n, &d);
    let (n, d) = if g.is_one() { (n, d) } else { (n / &g, d / &g) };
    if d.is_negative() { R::new_raw(-n, -d) } else { R::new_raw(n, d) }
}

pub fn add(a: &R, b: &R) -> R {
    ratio(a.numer() * b.denom() + b.numer() * a.denom(), a.denom() * b.denom())
}

pub fn sub(a: &R, b: &R) -> R {
    ratio(a.numer() * b.denom() - b.numer() * a.denom(), a.denom() * b.denom())
}

pub fn mul(a: &R, b: &R) -> R {
    ratio(a.numer() * b.numer(), a.denom() * b.denom())
}

/// `b` must not be zero.
pub fn div(a: &R, b: &R) -> R {
    ratio(a.numer() * b.denom(), a.denom() * b.numer())
}

/// π to 60 digits, far beyond any displayed precision.
pub fn pi() -> R {
    parse_decimal("3.14159265358979323846264338327950288419716939937510582097494").unwrap()
}

/// "12.5e-3", "-.5", "1,5" → exact rational.
pub fn parse_decimal(s: &str) -> Result<R, EvalError> {
    let s = s.trim().replace(',', ".");
    let (neg, s) = match s.strip_prefix('-') {
        Some(r) => (true, r.to_string()),
        None => (false, s.strip_prefix('+').unwrap_or(&s).to_string()),
    };
    let (mant, exp) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], s[i + 1..].parse::<i64>().map_err(|_| EvalError::Incomplete)?),
        None => (s.as_str(), 0),
    };
    let (ip, fp) = mant.split_once('.').unwrap_or((mant, ""));
    if ip.is_empty() && fp.is_empty() {
        return Err(EvalError::Incomplete);
    }
    if !ip.bytes().chain(fp.bytes()).all(|b| b.is_ascii_digit()) {
        return Err(EvalError::Invalid("Not a number".into()));
    }
    let n: BigInt = format!("0{ip}{fp}").parse().unwrap();
    let e = exp.saturating_sub(fp.len() as i64);
    if !n.is_zero() && (exp.abs() > MAX_EXP || e.abs() > MAX_EXP + 1000) {
        return Err(EvalError::Invalid(format!("Exponent too large (limit ±{MAX_EXP})")));
    }
    let q = if e >= 0 { R::from_integer(n * ten(e)) } else { ratio(n, ten(e)) };
    Ok(if neg { -q } else { q })
}

/// Exact value of an f64 *as written* (shortest round-trip decimal), e.g. API rates.
pub fn from_f64(v: f64) -> Option<R> {
    v.is_finite().then(|| parse_decimal(&format!("{v:e}")).ok()).flatten()
}

/// A floating-point result, rounded to the 15 significant digits an f64 actually carries
/// (so ln 1000 ÷ ln 10 gives 3, not 2.9999999999999996).
pub fn from_f64_rounded(v: f64) -> Option<R> {
    v.is_finite().then(|| parse_decimal(&format!("{v:.14e}")).ok()).flatten()
}

/// Rough decimal exponent check, so absurd values are rejected before formatting.
pub fn in_range(q: &R) -> bool {
    let bits = q.numer().bits() as i64 - q.denom().bits() as i64;
    q.is_zero() || bits.abs() < (MAX_EXP as f64 * 3.33) as i64
}

/// Evaluates `+ - * / ^ ( )` over exact decimals. `consts` resolves names (catalog only).
pub fn eval(s: &str, consts: &dyn Fn(&str) -> Option<R>) -> Result<R, EvalError> {
    let mut p = Parser { s: s.chars().collect(), i: 0, consts };
    let v = p.expr()?;
    match p.peek() {
        None if in_range(&v) => Ok(v),
        None => Err(EvalError::Invalid("Number too large".into())),
        Some(c) => Err(EvalError::Invalid(format!("Unexpected \"{c}\""))),
    }
}

/// Evaluate user input (no names).
pub fn eval_input(s: &str) -> Result<R, EvalError> {
    eval(s, &|_| None)
}

/// Input filter for numeric fields: digits, operators, parentheses; letters only as exponent "e".
pub fn accepts_number_input(s: &str) -> bool {
    let c: Vec<char> = s.chars().collect();
    c.len() <= 400
        && c.iter().enumerate().all(|(i, &ch)| match ch {
            '0'..='9' | '.' | ',' | ':' | '+' | '-' | '*' | '/' | '^' | '(' | ')' | ' ' | '×' | '÷' | '−' => true,
            'e' | 'E' => i > 0 && (c[i - 1].is_ascii_digit() || c[i - 1] == '.'),
            _ => false,
        })
}

/// Cleans pasted numbers: "1 234,5" → "1234,5", "1,234.5" → "1234.5", "−3" → "-3".
pub fn sanitize_number(s: &str) -> String {
    let s: String = s
        .trim()
        .chars()
        .filter(|c| !matches!(c, '_' | '\'' | '\u{a0}' | '\u{202f}' | '\u{2009}'))
        .map(|c| match c {
            '−' => '-',
            '×' => '*',
            '÷' => '/',
            _ => c,
        })
        .collect();
    let s = if s.contains('.') && s.contains(',') { s.replace(',', "") } else { s };
    // Spaces between digits are thousands separators; keep spaces around operators.
    let c: Vec<char> = s.chars().collect();
    c.iter()
        .enumerate()
        .filter(|&(i, &ch)| !(ch == ' ' && i > 0 && c[i - 1].is_ascii_digit() && c.get(i + 1).is_some_and(|n| n.is_ascii_digit())))
        .map(|(_, &ch)| ch)
        .collect()
}

struct Parser<'a> {
    s: Vec<char>,
    i: usize,
    consts: &'a dyn Fn(&str) -> Option<R>,
}

impl Parser<'_> {
    fn peek(&mut self) -> Option<char> {
        while self.s.get(self.i).is_some_and(|c| c.is_whitespace()) {
            self.i += 1;
        }
        self.s.get(self.i).copied()
    }

    fn expr(&mut self) -> Result<R, EvalError> {
        let mut v = self.term()?;
        loop {
            match self.peek() {
                Some('+') => {
                    self.i += 1;
                    v = add(&v, &self.term()?);
                }
                Some('-' | '−') => {
                    self.i += 1;
                    v = sub(&v, &self.term()?);
                }
                _ => return Ok(v),
            }
        }
    }

    fn term(&mut self) -> Result<R, EvalError> {
        let mut v = self.unary()?;
        loop {
            match self.peek() {
                Some('*' | '×') => {
                    self.i += 1;
                    v = mul(&v, &self.unary()?);
                }
                Some('/' | '÷') => {
                    self.i += 1;
                    let d = self.unary()?;
                    if d.is_zero() {
                        return Err(EvalError::Invalid("Division by zero".into()));
                    }
                    v = div(&v, &d);
                }
                _ => return Ok(v),
            }
        }
    }

    fn unary(&mut self) -> Result<R, EvalError> {
        match self.peek() {
            Some('-' | '−') => {
                self.i += 1;
                Ok(-self.unary()?)
            }
            Some('+') => {
                self.i += 1;
                self.unary()
            }
            _ => self.power(),
        }
    }

    fn power(&mut self) -> Result<R, EvalError> {
        let base = self.atom()?;
        if self.peek() != Some('^') {
            return Ok(base);
        }
        self.i += 1;
        let exp = self.unary()?;
        if !exp.is_integer() {
            return Err(EvalError::Invalid("Only whole-number powers are supported".into()));
        }
        let e = exp.to_integer().to_i64().filter(|e| e.abs() <= MAX_EXP).ok_or(EvalError::Invalid("Power too large".into()))?;
        let bits = (base.numer().bits() + base.denom().bits()) as i64;
        if bits * e.abs() > 400_000 {
            return Err(EvalError::Invalid("Number too large".into()));
        }
        if e < 0 && base.is_zero() {
            return Err(EvalError::Invalid("Division by zero".into()));
        }
        Ok(Pow::pow(base, e as i32))
    }

    fn atom(&mut self) -> Result<R, EvalError> {
        match self.peek() {
            None => Err(EvalError::Incomplete),
            Some('(') => {
                self.i += 1;
                let v = self.expr()?;
                match self.peek() {
                    Some(')') => {
                        self.i += 1;
                        Ok(v)
                    }
                    None => Err(EvalError::Incomplete),
                    Some(c) => Err(EvalError::Invalid(format!("Unexpected \"{c}\""))),
                }
            }
            Some(c) if c.is_ascii_digit() || c == '.' || c == ',' => self.number(),
            Some(c) if c.is_ascii_alphabetic() => {
                let start = self.i;
                while self.s.get(self.i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == '_') {
                    self.i += 1;
                }
                let name: String = self.s[start..self.i].iter().collect();
                (self.consts)(&name).ok_or(EvalError::Invalid(format!("Unknown name \"{name}\"")))
            }
            Some(c) => Err(EvalError::Invalid(format!("Unexpected \"{c}\""))),
        }
    }

    fn number(&mut self) -> Result<R, EvalError> {
        let start = self.i;
        let digit = |p: &Self, i: usize| p.s.get(i).is_some_and(|c| c.is_ascii_digit());
        while self.s.get(self.i).is_some_and(|&c| c.is_ascii_digit() || c == '.' || c == ',') {
            self.i += 1;
        }
        if matches!(self.s.get(self.i), Some('e' | 'E')) {
            let mut j = self.i + 1;
            if matches!(self.s.get(j), Some('+' | '-')) {
                j += 1;
            }
            if !digit(self, j) {
                return Err(if j >= self.s.len() { EvalError::Incomplete } else { EvalError::Invalid("Bad exponent".into()) });
            }
            while digit(self, j) {
                j += 1;
            }
            self.i = j;
        }
        let text: String = self.s[start..self.i].iter().collect();
        if text.matches(['.', ',']).count() > 1 {
            return Err(EvalError::Invalid("Too many decimal points".into()));
        }
        let mut v = parse_decimal(&text)?;
        // Sexagesimal "1:30" = 1.5, "12:30:15" = 12.504166… (h:m:s, °:′:″, pace m:ss).
        let mut unit = int(1);
        for _ in 0..2 {
            if self.s.get(self.i) != Some(&':') {
                break;
            }
            self.i += 1;
            let start = self.i;
            while self.s.get(self.i).is_some_and(|&c| c.is_ascii_digit() || c == '.' || c == ',') {
                self.i += 1;
            }
            if start == self.i {
                return Err(if self.i >= self.s.len() { EvalError::Incomplete } else { EvalError::Invalid("Expected digits after \":\"".into()) });
            }
            let part = parse_decimal(&self.s[start..self.i].iter().collect::<String>())?;
            if part >= int(60) {
                return Err(EvalError::Invalid("Minutes and seconds after \":\" must be below 60".into()));
            }
            unit = mul(&unit, &int(60));
            v = add(&v, &div(&part, &unit));
        }
        Ok(v)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Notation {
    /// Plain digits, scientific only for very large/small values.
    Auto,
    Scientific,
    /// Scientific with exponents in multiples of 3.
    Engineering,
    /// Always every digit, however many.
    Plain,
}

/// Rounds |q| to `sig` significant digits: (digits, decimal exponent of the first digit, exact?).
/// Integer-only (no fraction reductions), so it stays fast for huge values.
fn round_sig(a: &R, sig: usize) -> (BigInt, i64, bool) {
    let (n, d) = (a.numer(), a.denom());
    // a ≥ 10^e ⇔ n ≥ d·10^e
    let at_least = |e: i64| if e >= 0 { *n >= d * ten(e) } else { n * ten(e) >= *d };
    let bits = n.bits() as i64 - d.bits() as i64;
    let mut e = (bits as f64 * std::f64::consts::LOG10_2).floor() as i64;
    while !at_least(e) {
        e -= 1;
    }
    while at_least(e + 1) {
        e += 1;
    }
    let k = sig as i64 - 1 - e;
    let (num, den) = if k >= 0 { (n * ten(k), d.clone()) } else { (n.clone(), d * ten(k)) };
    let (mut q, r) = num.div_rem(&den);
    let exact = r.is_zero();
    if r * 2 >= den {
        q += 1;
    }
    if q == ten(sig as i64) {
        q /= 10;
        e += 1;
    }
    (q, e, exact)
}

/// Decimal text for `q` with `sig` significant digits. Also says whether it is exact.
/// The text is plain ASCII and can be parsed back (`1.5e20`, `-0.0003`).
pub fn format(q: &R, sig: usize, notation: Notation) -> (String, bool) {
    if q.is_zero() {
        return ("0".into(), true);
    }
    let sig = sig.clamp(1, 60);
    let (n, e, exact) = round_sig(&q.abs(), sig);
    let mut digits = n.to_string();
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }
    let sign = if q.is_negative() { "-" } else { "" };
    let sci = |shift: i64| {
        // `shift` extra digits before the point (engineering notation).
        let mut d = digits.clone();
        while (d.len() as i64) < shift + 1 {
            d.push('0');
        }
        let (int, frac) = d.split_at(shift as usize + 1);
        let frac = if frac.is_empty() { String::new() } else { format!(".{frac}") };
        format!("{sign}{int}{frac}e{}", e - shift)
    };
    let text = match notation {
        Notation::Scientific => sci(0),
        Notation::Engineering => sci(e.rem_euclid(3)),
        Notation::Auto if !(-7..21).contains(&e) => sci(0),
        _ if e >= 0 => {
            let mut d = digits.clone();
            while (d.len() as i64) < e + 1 {
                d.push('0');
            }
            let (int, frac) = d.split_at(e as usize + 1);
            if frac.is_empty() { format!("{sign}{int}") } else { format!("{sign}{int}.{frac}") }
        }
        _ => format!("{sign}0.{}{digits}", "0".repeat((-e - 1) as usize)),
    };
    (text, exact)
}

/// Fixed decimals, rounded half away from zero (money).
pub fn format_fixed(q: &R, decimals: usize) -> String {
    let (num, den) = (q.numer().abs() * ten(decimals as i64), q.denom());
    let (mut n, r) = num.div_rem(den);
    if r * 2 >= *den {
        n += 1;
    }
    let s = format!("{:0>width$}", n.to_string(), width = decimals + 1);
    let (int, frac) = s.split_at(s.len() - decimals);
    let sign = if q.is_negative() && !n.is_zero() { "-" } else { "" };
    if decimals == 0 { format!("{sign}{int}") } else { format!("{sign}{int}.{frac}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> String {
        format(&eval_input(s).unwrap(), 12, Notation::Auto).0
    }

    #[test]
    fn evaluates_exactly() {
        assert_eq!(ev("0.1+0.2"), "0.3");
        assert_eq!(ev("1/3*3"), "1");
        assert_eq!(ev("2*(3+4)^2"), "98");
        assert_eq!(ev("-2^2"), "-4");
        assert_eq!(ev("1,5*2"), "3");
        assert_eq!(ev("1e3/8"), "125");
        assert_eq!(ev("2^-2"), "0.25");
        assert_eq!(eval_input("2*"), Err(EvalError::Incomplete));
        assert_eq!(eval_input("1e"), Err(EvalError::Incomplete));
        assert_eq!(eval_input("(1+2"), Err(EvalError::Incomplete));
        assert!(matches!(eval_input("1/0"), Err(EvalError::Invalid(_))));
        assert!(matches!(eval_input("1e99999"), Err(EvalError::Invalid(_))));
        assert!(matches!(eval_input("2)"), Err(EvalError::Invalid(_))));
        assert_eq!(ev("1:30"), "1.5");
        assert_eq!(ev("-5:30"), "-5.5");
        assert_eq!(ev("12:30:36"), "12.51");
        assert_eq!(ev("1:30*2"), "3");
        assert_eq!(eval_input("1:"), Err(EvalError::Incomplete));
        assert!(matches!(eval_input("1:75"), Err(EvalError::Invalid(_))));
    }

    #[test]
    fn formats() {
        let f = |s: &str, sig, n| format(&eval_input(s).unwrap(), sig, n).0;
        assert_eq!(f("1/3", 5, Notation::Auto), "0.33333");
        assert_eq!(f("2/3", 3, Notation::Auto), "0.667");
        assert_eq!(f("999.96", 4, Notation::Auto), "1000");
        assert_eq!(f("1.5e30", 10, Notation::Auto), "1.5e30");
        assert_eq!(f("1.5e30", 10, Notation::Plain), "1500000000000000000000000000000");
        assert_eq!(f("-0.00012", 10, Notation::Auto), "-0.00012");
        assert_eq!(f("12345", 10, Notation::Scientific), "1.2345e4");
        assert_eq!(f("12345", 10, Notation::Engineering), "12.345e3");
        assert_eq!(f("1e-9", 10, Notation::Auto), "1e-9");
        assert_eq!(f("1e5000", 3, Notation::Auto), "1e5000");
        assert!(format(&eval_input("1/3").unwrap(), 10, Notation::Auto).1 == false);
        assert!(format(&eval_input("0.25").unwrap(), 10, Notation::Auto).1);
        assert_eq!(format_fixed(&eval_input("2.675").unwrap(), 2), "2.68");
        assert_eq!(format_fixed(&eval_input("-0.001").unwrap(), 2), "0.00");
    }

    #[test]
    fn input_filter_and_paste() {
        assert!(accepts_number_input("1.5e-3*(2+4)/7"));
        assert!(!accepts_number_input("12a") && !accepts_number_input("e5") && !accepts_number_input("pi"));
        assert_eq!(sanitize_number(" 1 234,5 "), "1234,5");
        assert_eq!(sanitize_number("1,234.5"), "1234.5");
        assert_eq!(sanitize_number("2 × 3"), "2 * 3");
        assert_eq!(from_f64(0.1).unwrap(), R::new(1.into(), 10.into()));
    }
}
