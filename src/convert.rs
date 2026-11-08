//! Conversion engine: turns an input string in one unit into a value in another, exactly.

use crate::catalog::{chem, music, numbers, sizes, zones, Kind, Unit};
use crate::num::{self, EvalError, Notation, R};
use crate::rates::Rates;
use num_traits::{Signed, ToPrimitive, Zero};

#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// Input is empty or still incomplete ("-", "1e", "2*"): show nothing, no error.
    Empty,
    Number(R),
    Text(String),
}

/// Evaluate numeric input (plain numbers or arithmetic). None while incomplete.
pub fn number(input: &str) -> Result<Option<R>, String> {
    match num::eval_input(input) {
        Ok(v) => Ok(Some(v)),
        Err(EvalError::Incomplete) => Ok(None),
        Err(EvalError::Invalid(m)) => Err(m),
    }
}

/// A value typed in a unit's own notation: number/expression, mixed number, date or duration.
pub fn parse_quantity(kind: Kind, input: &str) -> Result<Option<R>, String> {
    match kind {
        Kind::Date(f) => f.parse(input),
        Kind::Duration(f) => f.parse(input),
        Kind::Fraction(numbers::Frac::Mixed) => number(&numbers::mixed_to_expr(input)),
        Kind::Zone(i) => zones::parse(i, input),
        Kind::Size(t, c) => sizes::parse(t, c, input),
        Kind::Note => music::parse_note(input),
        Kind::Interval => music::parse_interval(input),
        Kind::Code(f) => f.parse(input),
        _ => number(input),
    }
}

/// The text of a value in units shown as words (dates, places' times, sizes, notes…).
fn words(kind: Kind, q: &R) -> Option<Result<String, String>> {
    Some(match kind {
        Kind::Date(f) => f.format(q),
        Kind::Zone(i) => zones::format(i, q),
        Kind::Size(t, c) => sizes::format(t, c, q),
        Kind::Note => music::format_note(q),
        Kind::Interval => music::format_interval(q),
        Kind::Code(f) => f.format(q),
        _ => return None,
    })
}

fn to_f64(q: &R) -> Result<f64, String> {
    q.to_f64().filter(|f| f.is_finite()).ok_or("Value out of range for this scale".into())
}

fn from_f64(v: f64) -> Result<R, String> {
    num::from_f64_rounded(v).ok_or("Value out of range for this scale".into())
}

pub fn to_base(u: &Unit, v: R) -> Result<R, String> {
    Ok(match u.kind {
        Kind::Affine => {
            let k = num::mul(&num::add(&v, &u.offset), &u.factor);
            if k.is_negative() && u.id.starts_with("temperature:") {
                return Err("Below absolute zero".into());
            }
            k
        }
        Kind::Scale(s) => from_f64(s.to_base(to_f64(&v)?))?,
        Kind::Reciprocal if v.is_zero() => return Err("Can't divide by zero".into()),
        Kind::Reciprocal => num::div(&u.factor, &v),
        _ => num::mul(&v, &u.factor),
    })
}

pub fn from_base(u: &Unit, base: R) -> Result<R, String> {
    Ok(match u.kind {
        Kind::Scale(s) => from_f64(s.base_to_unit(to_f64(&base)?)?)?,
        Kind::Affine => num::sub(&num::div(&base, &u.factor), &u.offset),
        Kind::Reciprocal if base.is_zero() => return Err("Result is infinite".into()),
        Kind::Reciprocal => num::div(&u.factor, &base),
        _ => num::div(&base, &u.factor),
    })
}

pub fn convert(from: &Unit, to: &Unit, input: &str, rates: Option<&Rates>) -> Result<Output, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(Output::Empty);
    }
    use Kind::*;
    let out = match (from.kind, to.kind) {
        (Text(a), Text(b)) => Output::Text(b.encode(&a.decode(input)?)),
        (Color(a), Color(b)) => match a.parse(input)? {
            Some(c) => Output::Text(b.format(c)),
            None => Output::Empty,
        },
        (Radix(_) | Roman, _) => {
            let n = match from.kind {
                Radix(r) => match numbers::parse_radix(input, r)? {
                    Some(n) => n,
                    None => return Ok(Output::Empty),
                },
                _ => numbers::parse_roman(input)?,
            };
            Output::Text(match to.kind {
                Radix(r) => numbers::to_radix(&n, r),
                Roman => numbers::to_roman(&n)?,
                Words => numbers::to_words(&n)?,
                _ => return Err("Incompatible units".into()),
            })
        }
        (Currency, Currency) => {
            let Some(v) = number(input)? else { return Ok(Output::Empty) };
            let rates = rates.ok_or("Exchange rates are not loaded yet")?;
            let rate = |u: &Unit| rates.get(&u.symbol).ok_or(format!("No rate available for {}", u.symbol));
            Output::Number(num::mul(&num::div(&v, &rate(from)?), &rate(to)?))
        }
        (Chem(_), Chem(b)) => chem::convert(b, input, 12)?.unwrap_or(Output::Empty),
        (Coord(a), Coord(b)) => match a.parse(input)? {
            Some(p) => Output::Text(b.format(p)),
            None => Output::Empty,
        },
        (a, b) if a.is_quantity() && b.is_quantity() => {
            let Some(v) = parse_quantity(a, input)? else { return Ok(Output::Empty) };
            let v = from_base(to, to_base(from, v)?)?;
            if let Some(text) = words(b, &v) {
                text?;
            }
            Output::Number(v)
        }
        _ => return Err("Incompatible units".into()),
    };
    match &out {
        Output::Number(v) if !num::in_range(v) => Err("Result is too large to show".into()),
        _ => Ok(out),
    }
}

/// Display text and exactness for a number in a unit. Money gets cents; everything else
/// `sig` significant digits.
pub fn format_value(kind: Kind, q: &R, sig: usize, notation: Notation) -> (String, bool) {
    if let Kind::Fraction(f) = kind {
        (numbers::format_fraction(q, f), true)
    } else if let Some(text) = words(kind, q) {
        (text.unwrap_or_else(|e| e), true)
    } else if let Kind::Duration(f) = kind {
        (f.format(q), true)
    } else if kind == Kind::Currency && q.abs() >= num::int(1) && q.abs() < num::pow10(15) {
        let cents_exact = (q.numer() * num_bigint::BigInt::from(100) % q.denom()).is_zero();
        (num::format_fixed(q, 2), cents_exact)
    } else if kind == Kind::Currency {
        num::format(q, sig.min(4), notation)
    } else {
        num::format(q, sig, notation)
    }
}

/// Digit grouping for display: `sep` between thousands, `dec` as decimal mark.
pub fn group_digits(s: &str, sep: char, dec: char) -> String {
    if s.contains('e') {
        return s.replace('.', &dec.to_string());
    }
    let (sign, rest) = s.strip_prefix('-').map_or(("", s), |r| ("-", r));
    let (int, frac) = rest.split_once('.').map_or((rest, None), |(i, f)| (i, Some(f)));
    let mut out = String::new();
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(c);
    }
    match frac {
        Some(f) => format!("{sign}{out}{dec}{f}"),
        None => format!("{sign}{out}"),
    }
}

/// "× 100", "÷ 1000", "× 1.8 + 32" — how to get `to` from `from`, when it's a simple rule.
pub fn formula(from: &Unit, to: &Unit, sig: usize) -> Option<String> {
    let f = |q: &R| num::format(q, sig, Notation::Auto).0;
    match (from.kind, to.kind) {
        (Kind::Linear, Kind::Linear) => {
            let k = &from.factor / &to.factor;
            Some(if k >= num::int(1) { format!("× {}", f(&k)) } else { format!("÷ {}", f(&(num::int(1) / k))) })
        }
        (Kind::Affine, Kind::Affine) => {
            let a = &from.factor / &to.factor;
            let b = &from.offset * &from.factor / &to.factor - &to.offset;
            let mul = if a == num::int(1) { String::new() } else { format!("× {} ", f(&a)) };
            Some(match b.is_negative() {
                true => format!("{mul}− {}", f(&-b)),
                false if b.is_zero() => mul.to_string().trim_end().to_string(),
                false => format!("{mul}+ {}", f(&b)),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;

    fn conv(c: &Catalog, from: &str, to: &str, input: &str) -> String {
        let (f, t) = (&c.units[c.find(from).unwrap()], &c.units[c.find(to).unwrap()]);
        match convert(f, t, input, None).unwrap() {
            Output::Number(v) => num::format(&v, 15, Notation::Auto).0,
            Output::Text(s) => s,
            Output::Empty => String::new(),
        }
    }

    #[test]
    fn physical_is_exact() {
        let c = Catalog::new();
        assert_eq!(conv(&c, "length:meter", "length:centimeter", "1"), "100");
        assert_eq!(conv(&c, "length:mile", "length:inch", "1"), "63360");
        assert_eq!(conv(&c, "length:mile", "length:kilometer", "1"), "1.609344");
        assert_eq!(conv(&c, "length:foot", "length:meter", "0.1+0.2"), "0.09144");
        assert_eq!(conv(&c, "temperature:celsius", "temperature:fahrenheit", "100"), "212");
        assert_eq!(conv(&c, "temperature:fahrenheit", "temperature:celsius", "-40"), "-40");
        assert_eq!(conv(&c, "temperature:delisle", "temperature:kelvin", "0"), "373.15");
        assert_eq!(conv(&c, "fuel:liter-per-100-km", "fuel:kilometer-per-liter", "5"), "20");
        assert_eq!(conv(&c, "data:kibibyte", "data:byte", "1"), "1024");
        assert_eq!(conv(&c, "volume:us-gallon", "volume:cubic-inch", "1"), "231");
        assert_eq!(conv(&c, "angle:turn", "angle:degree", "1"), "360");
        assert_eq!(conv(&c, "length:meter", "length:centimeter", "1,5"), "150");
        assert_eq!(conv(&c, "length:meter", "length:centimeter", "2*"), "");
        assert_eq!(conv(&c, "length:meter", "length:kilometer", "1e5000"), "1e4997");
        assert_eq!(conv(&c, "ratio:percent", "ratio:basis-point", "1.5"), "150");
        assert_eq!(conv(&c, "numbers:hexadecimal--base-16-", "numbers:roman-numerals", "7EA"), "MMXXVI");
        assert_eq!(conv(&c, "color:hex-color", "color:rgb", "#fff"), "rgb(255, 255, 255)");
        let (cel, k) = (&c.units[c.find("temperature:celsius").unwrap()], &c.units[c.find("temperature:kelvin").unwrap()]);
        assert!(convert(cel, k, "-300", None).is_err());
    }

    #[test]
    fn formulas() {
        let c = Catalog::new();
        let u = |id: &str| &c.units[c.find(id).unwrap()];
        assert_eq!(formula(u("length:meter"), u("length:centimeter"), 10).unwrap(), "× 100");
        assert_eq!(formula(u("length:centimeter"), u("length:meter"), 10).unwrap(), "÷ 100");
        assert_eq!(formula(u("temperature:celsius"), u("temperature:fahrenheit"), 10).unwrap(), "× 1.8 + 32");
        assert_eq!(formula(u("temperature:kelvin"), u("temperature:celsius"), 10).unwrap(), "− 273.15");
    }

    #[test]
    fn currency_uses_rates() {
        let c = Catalog::new();
        let (usd, uah) = (&c.units[c.find("currency:us-dollar").unwrap()], &c.units[c.find("currency:ukrainian-hryvnia").unwrap()]);
        assert!(convert(usd, uah, "1", None).is_err());
        let rates = Rates::from_pairs(&[("usd", 1.0), ("uah", 41.5)]);
        let Ok(Output::Number(v)) = convert(usd, uah, "2", Some(&rates)) else { panic!() };
        assert_eq!(format_value(Kind::Currency, &v, 10, Notation::Auto), ("83.00".to_string(), true));
    }

    #[test]
    fn grouping() {
        assert_eq!(group_digits("-1234567.891", ',', '.'), "-1,234,567.891");
        assert_eq!(group_digits("123", ' ', ','), "123");
        assert_eq!(group_digits("1.5e30", ',', ','), "1,5e30");
    }
}

#[cfg(test)]
mod bench {
    /// `cargo test --release huge_table_speed -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn huge_table_speed() {
        let c = crate::catalog::Catalog::new();
        let m = &c.units[c.find("length:meter").unwrap()];
        let t = std::time::Instant::now();
        for &u in &c.categories[m.category].units {
            if let Ok(super::Output::Number(v)) = super::convert(m, &c.units[u], "1e5000+0.5", None) {
                crate::num::format(&v, 12, crate::num::Notation::Auto);
            }
        }
        eprintln!("table: {:?}", t.elapsed());
    }
}
