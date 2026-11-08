//! Calculators: small forms with one to four inputs and one result. Each opens directly from
//! the picker. Adding one = one entry in `CALCS` (and a group in `GROUPS` if it's a new topic).
//!
//! Results are exact where the math allows; roots, logs and non-integer powers use floating
//! point and show as "≈".

use super::time::{self, DateFmt, DurFmt};
use super::{Builder, Kind};
use crate::num::{self, int, R};
use num_bigint::BigInt;
use num_traits::{One, Signed, ToPrimitive, Zero};

pub struct In {
    pub label: &'static str,
    /// Starting value; "today" is replaced by today's date.
    pub default: &'static str,
    pub kind: Kind,
}

const fn n(label: &'static str, default: &'static str) -> In {
    In { label, default, kind: Kind::Linear }
}

const fn date(label: &'static str, default: &'static str) -> In {
    In { label, default, kind: Kind::Date(DateFmt::Iso) }
}

pub struct Calc {
    pub name: &'static str,
    pub symbol: &'static str,
    pub keywords: &'static str,
    /// Index into `GROUPS`.
    pub group: usize,
    pub inputs: &'static [In],
    pub result: &'static str,
    /// Shown after the result: "%", " Ω" or "".
    pub suffix: &'static str,
    /// How the result is shown (a number, a date, a duration…).
    pub out: Kind,
    pub formula: &'static str,
    pub compute: fn(&[R]) -> Result<R, &'static str>,
}

/// (category id, name). Ids are part of saved favorite keys: don't rename them.
pub const GROUPS: &[(&str, &str)] = &[
    ("percent", "Percentage Calculators"),
    ("mathcalc", "Math Calculators"),
    ("eleccalc", "Electronics Calculators"),
    ("chemcalc", "Chemistry Calculators"),
    ("everyday", "Everyday & Money"),
    ("healthcalc", "Health Calculators"),
    ("photocalc", "Photography Calculators"),
    ("musiccalc", "Music & Audio Calculators"),
    ("bizcalc", "Business & Finance"),
    ("physcalc", "Physics Calculators"),
    ("devcalc", "Programming & Servers"),
    ("gamecalc", "Gaming & Displays"),
    ("homecalc", "Home & DIY"),
];
const PCT: usize = 0;
const MATH: usize = 1;
const ELEC: usize = 2;
const CHEM: usize = 3;
const DAY: usize = 4;
const HEALTH: usize = 5;
const PHOTO: usize = 6;
const MUSIC: usize = 7;
const BIZ: usize = 8;
const PHYS: usize = 9;
const DEV: usize = 10;
const GAME: usize = 11;
const HOME: usize = 12;

fn pct(p: &R) -> R {
    p / int(100)
}

fn nonzero(v: &R, msg: &'static str) -> Result<(), &'static str> {
    if v.is_zero() { Err(msg) } else { Ok(()) }
}

fn positive(v: &R, msg: &'static str) -> Result<(), &'static str> {
    if v.is_positive() { Ok(()) } else { Err(msg) }
}

fn fl(q: &R) -> Result<f64, &'static str> {
    q.to_f64().filter(|f| f.is_finite()).ok_or("Value out of range")
}

fn from_fl(v: f64) -> Result<R, &'static str> {
    num::from_f64_rounded(v).ok_or("Result is not a finite number")
}

/// A whole number that fits the given limit.
fn whole(q: &R, max: i64, msg: &'static str) -> Result<i64, &'static str> {
    q.is_integer().then(|| q.to_integer().to_i64()).flatten().filter(|v| (0..=max).contains(v)).ok_or(msg)
}

/// Exact power for whole exponents (within limits), floating point otherwise.
fn power(base: &R, exp: &R) -> Result<R, &'static str> {
    if exp.is_integer() {
        let e = exp.to_integer().to_i64().filter(|e| e.abs() <= num::MAX_EXP).ok_or("Power too large")?;
        let bits = (base.numer().bits() + base.denom().bits()) as i64;
        if bits * e.abs() > 400_000 {
            return Err("Result too large");
        }
        if e < 0 && base.is_zero() {
            return Err("Can't divide by zero");
        }
        return Ok(num_traits::Pow::pow(base.clone(), e as i32));
    }
    if base.is_negative() {
        return Err("Negative base needs a whole-number power");
    }
    from_fl(fl(base)?.powf(fl(exp)?))
}

fn integer(q: &R, msg: &'static str) -> Result<BigInt, &'static str> {
    q.is_integer().then(|| q.to_integer()).ok_or(msg)
}

fn deg(q: &R) -> Result<f64, &'static str> {
    Ok(fl(q)?.to_radians())
}

/// A percentage that must lie in 0..=100, as a fraction.
fn share(q: &R, msg: &'static str) -> Result<f64, &'static str> {
    let p = fl(q)?;
    if (0.0..=100.0).contains(&p) { Ok(p / 100.0) } else { Err(msg) }
}

/// Standard gravity, m/s².
fn g0() -> R {
    super::expr("g0")
}

fn pi() -> R {
    super::expr("pi")
}

/// Gravitational constant, m³/(kg·s²) (CODATA 2018).
const BIG_G: f64 = 6.6743e-11;

/// Gas constant, J/(mol·K) (exact since the 2019 SI).
fn gas_r() -> R {
    num::parse_decimal("8.31446261815324").unwrap_or_default()
}

/// Full years from `born` (Unix seconds) to today, by calendar date.
fn age(born: &R) -> Result<R, &'static str> {
    let day = |v: &R| time::civil_from_days(v.floor().to_integer().to_i64().unwrap_or(0).div_euclid(86400));
    let (b, t) = (day(born), day(&time::now()));
    if b > t {
        return Err("Birth date is in the future");
    }
    Ok(int(t.0 - b.0 - i64::from((t.1, t.2) < (b.1, b.2))))
}

pub const CALCS: &[Calc] = &[
    // ── Percentages ──
    Calc {
        name: "X% of Y", symbol: "% of", keywords: "percent of percentage tip", group: PCT,
        inputs: &[n("Percent (%)", "10"), n("Of value", "200")], result: "Result", suffix: "", out: Kind::Linear,
        formula: "Y × X ÷ 100",
        compute: |v| Ok(&v[1] * pct(&v[0])),
    },
    Calc {
        name: "X is what % of Y", symbol: "is %", keywords: "percentage ratio share", group: PCT,
        inputs: &[n("Value", "10"), n("Of total", "200")], result: "Percentage", suffix: "%", out: Kind::Linear,
        formula: "X ÷ Y × 100",
        compute: |v| nonzero(&v[1], "Total can't be zero").map(|_| &v[0] / &v[1] * int(100)),
    },
    Calc {
        name: "Percentage change", symbol: "Δ%", keywords: "percent increase decrease growth difference from to", group: PCT,
        inputs: &[n("From", "80"), n("To", "100")], result: "Change", suffix: "%", out: Kind::Linear,
        formula: "(To − From) ÷ |From| × 100",
        compute: |v| nonzero(&v[0], "Starting value can't be zero").map(|_| (&v[1] - &v[0]) / v[0].abs() * int(100)),
    },
    Calc {
        name: "Increase by %", symbol: "+%", keywords: "percent add markup tax vat raise", group: PCT,
        inputs: &[n("Value", "100"), n("Increase (%)", "20")], result: "Result", suffix: "", out: Kind::Linear,
        formula: "Value × (1 + P ÷ 100)",
        compute: |v| Ok(&v[0] * (int(1) + pct(&v[1]))),
    },
    Calc {
        name: "Decrease by %", symbol: "−%", keywords: "percent discount subtract sale off", group: PCT,
        inputs: &[n("Value", "100"), n("Decrease (%)", "20")], result: "Result", suffix: "", out: Kind::Linear,
        formula: "Value × (1 − P ÷ 100)",
        compute: |v| Ok(&v[0] * (int(1) - pct(&v[1]))),
    },
    Calc {
        name: "X is P% of what", symbol: "÷%", keywords: "reverse percentage whole total", group: PCT,
        inputs: &[n("Value", "30"), n("Is percent (%)", "15")], result: "Whole", suffix: "", out: Kind::Linear,
        formula: "Value ÷ P × 100",
        compute: |v| nonzero(&v[1], "Percent can't be zero").map(|_| &v[0] / pct(&v[1])),
    },
    Calc {
        name: "Value before % increase", symbol: "pre+%", keywords: "reverse original price before tax vat net gross", group: PCT,
        inputs: &[n("Final value", "120"), n("Increase (%)", "20")], result: "Original", suffix: "", out: Kind::Linear,
        formula: "Final ÷ (1 + P ÷ 100)",
        compute: |v| {
            let d = int(1) + pct(&v[1]);
            nonzero(&d, "Increase can't be −100%").map(|_| &v[0] / d)
        },
    },
    Calc {
        name: "Value before % decrease", symbol: "pre−%", keywords: "reverse original price before discount", group: PCT,
        inputs: &[n("Final value", "80"), n("Decrease (%)", "20")], result: "Original", suffix: "", out: Kind::Linear,
        formula: "Final ÷ (1 − P ÷ 100)",
        compute: |v| {
            let d = int(1) - pct(&v[1]);
            nonzero(&d, "Decrease can't be 100%").map(|_| &v[0] / d)
        },
    },
    Calc {
        name: "Percentage difference", symbol: "%diff", keywords: "percent difference between symmetric compare", group: PCT,
        inputs: &[n("Value A", "90"), n("Value B", "110")], result: "Difference", suffix: "%", out: Kind::Linear,
        formula: "|A − B| ÷ ((A + B) ÷ 2) × 100",
        compute: |v| {
            let mean = (&v[0] + &v[1]) / int(2);
            nonzero(&mean, "Average can't be zero").map(|_| (&v[0] - &v[1]).abs() / mean.abs() * int(100))
        },
    },
    Calc {
        name: "Percentage points", symbol: "pp", keywords: "percentage point difference rate change", group: PCT,
        inputs: &[n("From (%)", "4.5"), n("To (%)", "5.25")], result: "Change", suffix: " pp", out: Kind::Linear,
        formula: "To − From",
        compute: |v| Ok(&v[1] - &v[0]),
    },
    Calc {
        name: "Profit margin", symbol: "margin", keywords: "gross margin profit percent business", group: PCT,
        inputs: &[n("Cost", "60"), n("Price", "100")], result: "Margin", suffix: "%", out: Kind::Linear,
        formula: "(Price − Cost) ÷ Price × 100",
        compute: |v| nonzero(&v[1], "Price can't be zero").map(|_| (&v[1] - &v[0]) / &v[1] * int(100)),
    },
    Calc {
        name: "Markup", symbol: "markup", keywords: "markup percent profit cost business", group: PCT,
        inputs: &[n("Cost", "60"), n("Price", "100")], result: "Markup", suffix: "%", out: Kind::Linear,
        formula: "(Price − Cost) ÷ Cost × 100",
        compute: |v| nonzero(&v[0], "Cost can't be zero").map(|_| (&v[1] - &v[0]) / &v[0] * int(100)),
    },
    Calc {
        name: "Price for target margin", symbol: "price", keywords: "selling price from margin cost business", group: PCT,
        inputs: &[n("Cost", "60"), n("Margin (%)", "40")], result: "Price", suffix: "", out: Kind::Linear,
        formula: "Cost ÷ (1 − M ÷ 100)",
        compute: |v| {
            let d = int(1) - pct(&v[1]);
            nonzero(&d, "Margin can't be 100%").map(|_| &v[0] / d)
        },
    },
    // ── Math ──
    Calc {
        name: "Greatest common divisor", symbol: "gcd", keywords: "gcd hcf gcf highest factor math", group: MATH,
        inputs: &[n("A", "84"), n("B", "36")], result: "GCD", suffix: "", out: Kind::Linear,
        formula: "Largest whole number dividing both",
        compute: |v| {
            let (a, b) = (integer(&v[0], "Whole numbers only")?, integer(&v[1], "Whole numbers only")?);
            Ok(R::from_integer(num::gcd(&a, &b)))
        },
    },
    Calc {
        name: "Least common multiple", symbol: "lcm", keywords: "lcm lowest smallest multiple denominator math", group: MATH,
        inputs: &[n("A", "4"), n("B", "6")], result: "LCM", suffix: "", out: Kind::Linear,
        formula: "|A × B| ÷ gcd(A, B)",
        compute: |v| {
            let (a, b) = (integer(&v[0], "Whole numbers only")?, integer(&v[1], "Whole numbers only")?);
            let g = num::gcd(&a, &b);
            if g.is_zero() { Ok(int(0)) } else { Ok(R::from_integer((a * b).abs() / g)) }
        },
    },
    Calc {
        name: "Factorial", symbol: "n!", keywords: "factorial math", group: MATH,
        inputs: &[n("n", "20")], result: "n!", suffix: "", out: Kind::Linear,
        formula: "1 × 2 × … × n",
        compute: |v| {
            let n = whole(&v[0], 3000, "Use a whole number from 0 to 3000")?;
            Ok(R::from_integer((1..=n).fold(BigInt::one(), |a, k| a * k)))
        },
    },
    Calc {
        name: "Combinations (n choose k)", symbol: "nCr", keywords: "combinations choose binomial coefficient lottery math", group: MATH,
        inputs: &[n("n (items)", "49"), n("k (chosen)", "6")], result: "Ways", suffix: "", out: Kind::Linear,
        formula: "n! ÷ (k! × (n − k)!)",
        compute: |v| {
            let n = whole(&v[0], 1_000_000, "n must be a whole number up to 1,000,000")?;
            let k = whole(&v[1], n, "k must be a whole number from 0 to n")?;
            let k = k.min(n - k);
            if k > 5000 {
                return Err("Too large (k and n − k both above 5000)");
            }
            let mut r = BigInt::one();
            for i in 0..k {
                r = r * (n - i) / (i + 1);
            }
            Ok(R::from_integer(r))
        },
    },
    Calc {
        name: "Permutations", symbol: "nPr", keywords: "permutations arrangements order math", group: MATH,
        inputs: &[n("n (items)", "10"), n("k (arranged)", "3")], result: "Ways", suffix: "", out: Kind::Linear,
        formula: "n! ÷ (n − k)!",
        compute: |v| {
            let n = whole(&v[0], 1_000_000, "n must be a whole number up to 1,000,000")?;
            let k = whole(&v[1], n, "k must be a whole number from 0 to n")?;
            if k > 5000 {
                return Err("Too large (k above 5000)");
            }
            Ok(R::from_integer((0..k).fold(BigInt::one(), |a, i| a * (n - i))))
        },
    },
    Calc {
        name: "Power", symbol: "xʸ", keywords: "power exponent raise math", group: MATH,
        inputs: &[n("Base x", "2"), n("Exponent y", "64")], result: "xʸ", suffix: "", out: Kind::Linear,
        formula: "x × x × … (y times); exact for whole y",
        compute: |v| power(&v[0], &v[1]),
    },
    Calc {
        name: "Root", symbol: "ʸ√x", keywords: "square cube root nth radical sqrt math", group: MATH,
        inputs: &[n("Value x", "2"), n("Degree (2 = square root)", "2")], result: "Root", suffix: "", out: Kind::Linear,
        formula: "x^(1/n)",
        compute: |v| {
            let (x, d) = (fl(&v[0])?, fl(&v[1])?);
            if d == 0.0 {
                return Err("Degree can't be zero");
            }
            let odd = v[1].is_integer() && (v[1].to_integer() % 2u32) != BigInt::zero();
            match x {
                x if x < 0.0 && odd => from_fl(-(-x).powf(1.0 / d)),
                x if x < 0.0 => Err("Even roots of negative numbers aren't real"),
                x => from_fl(x.powf(1.0 / d)),
            }
        },
    },
    Calc {
        name: "Logarithm", symbol: "log", keywords: "log logarithm ln natural base math", group: MATH,
        inputs: &[n("Value x", "1000"), n("Base (e ≈ 2.718281828)", "10")], result: "log", suffix: "", out: Kind::Linear,
        formula: "ln x ÷ ln base",
        compute: |v| {
            let (x, b) = (fl(&v[0])?, fl(&v[1])?);
            if x <= 0.0 || b <= 0.0 || b == 1.0 {
                return Err("Needs x > 0 and a base > 0 other than 1");
            }
            from_fl(x.ln() / b.ln())
        },
    },
    Calc {
        name: "Modulo (remainder)", symbol: "mod", keywords: "modulo remainder mod division math", group: MATH,
        inputs: &[n("Dividend", "17"), n("Divisor", "5")], result: "Remainder", suffix: "", out: Kind::Linear,
        formula: "a − b × ⌊a ÷ b⌋ (same sign as b)",
        compute: |v| {
            nonzero(&v[1], "Divisor can't be zero")?;
            Ok(&v[0] - &v[1] * (&v[0] / &v[1]).floor())
        },
    },
    Calc {
        name: "Hypotenuse", symbol: "√(a²+b²)", keywords: "pythagoras pythagorean triangle diagonal math", group: MATH,
        inputs: &[n("Side a", "3"), n("Side b", "4")], result: "Hypotenuse c", suffix: "", out: Kind::Linear,
        formula: "√(a² + b²)",
        compute: |v| from_fl(fl(&v[0])?.hypot(fl(&v[1])?)),
    },
    Calc {
        name: "Circle from radius", symbol: "πr²", keywords: "circle area radius pi math", group: MATH,
        inputs: &[n("Radius", "1")], result: "Area", suffix: "", out: Kind::Linear,
        formula: "π × r²  (circumference = 2πr)",
        compute: |v| Ok(num::pi() * &v[0] * &v[0]),
    },
    Calc {
        name: "Rule of three", symbol: "a:b=c:?", keywords: "proportion ratio scale recipe cross multiply math", group: MATH,
        inputs: &[n("a", "2"), n("is to b", "300"), n("as c is to ?", "5")], result: "?", suffix: "", out: Kind::Linear,
        formula: "b × c ÷ a",
        compute: |v| nonzero(&v[0], "a can't be zero").map(|_| &v[1] * &v[2] / &v[0]),
    },
    // ── Electronics ──
    Calc {
        name: "Ohm's law: voltage", symbol: "V=IR", keywords: "ohm voltage current resistance electronics", group: ELEC,
        inputs: &[n("Current (A)", "0.02"), n("Resistance (Ω)", "220")], result: "Voltage", suffix: " V", out: Kind::Linear,
        formula: "V = I × R",
        compute: |v| Ok(&v[0] * &v[1]),
    },
    Calc {
        name: "Ohm's law: current", symbol: "I=V/R", keywords: "ohm voltage current resistance amps electronics", group: ELEC,
        inputs: &[n("Voltage (V)", "5"), n("Resistance (Ω)", "220")], result: "Current", suffix: " A", out: Kind::Linear,
        formula: "I = V ÷ R",
        compute: |v| nonzero(&v[1], "Resistance can't be zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Ohm's law: resistance", symbol: "R=V/I", keywords: "ohm voltage current resistance electronics", group: ELEC,
        inputs: &[n("Voltage (V)", "5"), n("Current (A)", "0.02")], result: "Resistance", suffix: " Ω", out: Kind::Linear,
        formula: "R = V ÷ I",
        compute: |v| nonzero(&v[1], "Current can't be zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Electric power", symbol: "P=VI", keywords: "power watts voltage current electronics", group: ELEC,
        inputs: &[n("Voltage (V)", "230"), n("Current (A)", "10")], result: "Power", suffix: " W", out: Kind::Linear,
        formula: "P = V × I  (= I²R = V²/R)",
        compute: |v| Ok(&v[0] * &v[1]),
    },
    Calc {
        name: "Power from current and resistance", symbol: "P=I²R", keywords: "power watts current resistance heat electronics", group: ELEC,
        inputs: &[n("Current (A)", "2"), n("Resistance (Ω)", "10")], result: "Power", suffix: " W", out: Kind::Linear,
        formula: "P = I² × R",
        compute: |v| Ok(&v[0] * &v[0] * &v[1]),
    },
    Calc {
        name: "Power from voltage and resistance", symbol: "P=V²/R", keywords: "power watts voltage resistance electronics", group: ELEC,
        inputs: &[n("Voltage (V)", "12"), n("Resistance (Ω)", "6")], result: "Power", suffix: " W", out: Kind::Linear,
        formula: "P = V² ÷ R",
        compute: |v| nonzero(&v[1], "Resistance can't be zero").map(|_| &v[0] * &v[0] / &v[1]),
    },
    Calc {
        name: "Resistors in parallel", symbol: "R∥R", keywords: "parallel resistors equivalent resistance electronics", group: ELEC,
        inputs: &[n("R1 (Ω)", "100"), n("R2 (Ω)", "220")], result: "Equivalent", suffix: " Ω", out: Kind::Linear,
        formula: "R1 × R2 ÷ (R1 + R2)  (series: R1 + R2)",
        compute: |v| {
            let s = &v[0] + &v[1];
            nonzero(&s, "R1 + R2 can't be zero").map(|_| &v[0] * &v[1] / s)
        },
    },
    Calc {
        name: "Voltage divider", symbol: "Vout", keywords: "voltage divider resistors output electronics", group: ELEC,
        inputs: &[n("Input voltage (V)", "12"), n("R1, top (Ω)", "10000"), n("R2, bottom (Ω)", "4700")], result: "Output", suffix: " V", out: Kind::Linear,
        formula: "Vin × R2 ÷ (R1 + R2)",
        compute: |v| {
            let s = &v[1] + &v[2];
            nonzero(&s, "R1 + R2 can't be zero").map(|_| &v[0] * &v[2] / s)
        },
    },
    Calc {
        name: "LED series resistor", symbol: "R(LED)", keywords: "led resistor current limiting electronics", group: ELEC,
        inputs: &[n("Supply (V)", "5"), n("LED forward voltage (V)", "2"), n("LED current (mA)", "20")], result: "Resistor", suffix: " Ω", out: Kind::Linear,
        formula: "(Supply − LED) ÷ current",
        compute: |v| {
            positive(&v[2], "Current must be above zero")?;
            let drop = &v[0] - &v[1];
            positive(&drop, "Supply must be above the LED voltage")?;
            Ok(drop / (&v[2] / int(1000)))
        },
    },
    Calc {
        name: "RC time constant", symbol: "τ=RC", keywords: "rc time constant capacitor charge electronics", group: ELEC,
        inputs: &[n("Resistance (Ω)", "10000"), n("Capacitance (µF)", "100")], result: "τ", suffix: " s", out: Kind::Linear,
        formula: "τ = R × C  (≈ 63% charged after τ, ≈ 99% after 5τ)",
        compute: |v| Ok(&v[0] * &v[1] / int(1_000_000)),
    },
    Calc {
        name: "LC resonant frequency", symbol: "f₀", keywords: "lc resonance tank circuit frequency radio electronics", group: ELEC,
        inputs: &[n("Inductance (µH)", "10"), n("Capacitance (pF)", "100")], result: "Frequency", suffix: " Hz", out: Kind::Linear,
        formula: "1 ÷ (2π √(L × C))",
        compute: |v| {
            let lc = fl(&v[0])? * 1e-6 * fl(&v[1])? * 1e-12;
            if lc <= 0.0 {
                return Err("L and C must be above zero");
            }
            from_fl(1.0 / (2.0 * std::f64::consts::PI * lc.sqrt()))
        },
    },
    Calc {
        name: "Battery runtime", symbol: "mAh÷mA", keywords: "battery life runtime capacity hours electronics", group: ELEC,
        inputs: &[n("Capacity (mAh)", "3000"), n("Load (mA)", "250")], result: "Runtime", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "Capacity ÷ load (ideal; real batteries give less)",
        compute: |v| {
            positive(&v[1], "Load must be above zero")?;
            Ok(&v[0] / &v[1] * int(3600))
        },
    },
    // ── Chemistry ──
    Calc {
        name: "Dilution (C₁V₁ = C₂V₂)", symbol: "C₁V₁", keywords: "dilution concentration volume stock solution chemistry", group: CHEM,
        inputs: &[n("Stock concentration C₁", "2"), n("Stock volume V₁", "50"), n("Target concentration C₂", "0.5")], result: "Final volume V₂", suffix: "", out: Kind::Linear,
        formula: "V₂ = C₁ × V₁ ÷ C₂ (same units on both sides)",
        compute: |v| nonzero(&v[2], "Target concentration can't be zero").map(|_| &v[0] * &v[1] / &v[2]),
    },
    Calc {
        name: "Molarity from mass", symbol: "mol/L", keywords: "molarity concentration solution mass chemistry", group: CHEM,
        inputs: &[n("Mass (g)", "5.844"), n("Molar mass (g/mol)", "58.44"), n("Volume (L)", "0.1")], result: "Molarity", suffix: " mol/L", out: Kind::Linear,
        formula: "mass ÷ molar mass ÷ volume",
        compute: |v| {
            positive(&v[1], "Molar mass must be above zero")?;
            positive(&v[2], "Volume must be above zero")?;
            Ok(&v[0] / &v[1] / &v[2])
        },
    },
    Calc {
        name: "Mass for a solution", symbol: "g", keywords: "mass needed weigh molarity solution prepare chemistry", group: CHEM,
        inputs: &[n("Molarity (mol/L)", "1"), n("Molar mass (g/mol)", "58.44"), n("Volume (L)", "0.25")], result: "Mass", suffix: " g", out: Kind::Linear,
        formula: "molarity × molar mass × volume",
        compute: |v| Ok(&v[0] * &v[1] * &v[2]),
    },
    Calc {
        name: "Moles from mass", symbol: "n=m/M", keywords: "moles amount substance mass chemistry", group: CHEM,
        inputs: &[n("Mass (g)", "18"), n("Molar mass (g/mol)", "18.015")], result: "Amount", suffix: " mol", out: Kind::Linear,
        formula: "n = m ÷ M  (× 6.02214076e23 for particles)",
        compute: |v| positive(&v[1], "Molar mass must be above zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Ideal gas pressure", symbol: "P=nRT/V", keywords: "ideal gas law pressure pv nrt chemistry physics", group: CHEM,
        inputs: &[n("Amount (mol)", "1"), n("Temperature (K)", "273.15"), n("Volume (L)", "22.414")], result: "Pressure", suffix: " kPa", out: Kind::Linear,
        formula: "P = n × R × T ÷ V,  R = 8.31446261815324 J/(mol·K)",
        compute: |v| positive(&v[2], "Volume must be above zero").map(|_| &v[0] * gas_r() * &v[1] / &v[2]),
    },
    Calc {
        name: "Ideal gas volume", symbol: "V=nRT/P", keywords: "ideal gas law volume pv nrt chemistry physics", group: CHEM,
        inputs: &[n("Amount (mol)", "1"), n("Temperature (K)", "298.15"), n("Pressure (kPa)", "101.325")], result: "Volume", suffix: " L", out: Kind::Linear,
        formula: "V = n × R × T ÷ P,  R = 8.31446261815324 J/(mol·K)",
        compute: |v| positive(&v[2], "Pressure must be above zero").map(|_| &v[0] * gas_r() * &v[1] / &v[2]),
    },
    // ── Everyday & money ──
    Calc {
        name: "Tip and split", symbol: "tip", keywords: "tip gratuity split bill restaurant share people", group: DAY,
        inputs: &[n("Bill", "84.50"), n("Tip (%)", "15"), n("People", "3")], result: "Each pays", suffix: "", out: Kind::Currency,
        formula: "Bill × (1 + tip ÷ 100) ÷ people",
        compute: |v| positive(&v[2], "At least one person").map(|_| &v[0] * (int(1) + pct(&v[1])) / &v[2]),
    },
    Calc {
        name: "Unit price", symbol: "per unit", keywords: "unit price compare shopping per kg per item cost", group: DAY,
        inputs: &[n("Price", "3.49"), n("Quantity", "750")], result: "Price per unit", suffix: "", out: Kind::Linear,
        formula: "Price ÷ quantity (use the same unit to compare products)",
        compute: |v| nonzero(&v[1], "Quantity can't be zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Loan payment", symbol: "loan", keywords: "loan mortgage monthly payment annuity credit interest", group: DAY,
        inputs: &[n("Amount", "250000"), n("Yearly interest (%)", "5"), n("Years", "30")], result: "Monthly payment", suffix: "", out: Kind::Currency,
        formula: "A × r ÷ (1 − (1 + r)^−n),  r = rate ÷ 1200, n = years × 12",
        compute: |v| {
            let months = &v[2] * int(12);
            positive(&months, "Term must be above zero")?;
            let r = &v[1] / int(1200);
            if r.is_zero() {
                return Ok(&v[0] / months);
            }
            let growth = power(&(int(1) + &r), &months)?;
            Ok(&v[0] * &r * &growth / (growth - int(1)))
        },
    },
    Calc {
        name: "Compound interest", symbol: "A=P(1+r)ⁿ", keywords: "compound interest savings investment growth deposit", group: DAY,
        inputs: &[n("Principal", "10000"), n("Yearly rate (%)", "5"), n("Years", "10"), n("Compounds per year", "12")], result: "Final amount", suffix: "", out: Kind::Currency,
        formula: "P × (1 + r ÷ n)^(n × years)",
        compute: |v| {
            positive(&v[3], "Compounds per year must be above zero")?;
            Ok(&v[0] * power(&(int(1) + pct(&v[1]) / &v[3]), &(&v[3] * &v[2]))?)
        },
    },
    Calc {
        name: "Simple interest", symbol: "Prt", keywords: "simple interest savings loan", group: DAY,
        inputs: &[n("Principal", "10000"), n("Yearly rate (%)", "5"), n("Years", "3")], result: "Interest", suffix: "", out: Kind::Currency,
        formula: "P × r ÷ 100 × years",
        compute: |v| Ok(&v[0] * pct(&v[1]) * &v[2]),
    },
    Calc {
        name: "Trip fuel cost", symbol: "fuel $", keywords: "fuel petrol gas cost trip car drive distance", group: DAY,
        inputs: &[n("Distance (km)", "350"), n("Consumption (L/100 km)", "6.5"), n("Price per liter", "1.79")], result: "Fuel cost", suffix: "", out: Kind::Currency,
        formula: "Distance ÷ 100 × consumption × price  (fuel: distance × L/100km ÷ 100)",
        compute: |v| Ok(&v[0] / int(100) * &v[1] * &v[2]),
    },
    Calc {
        name: "Electricity cost", symbol: "kWh $", keywords: "electricity energy cost power appliance bill kwh", group: DAY,
        inputs: &[n("Power (W)", "2000"), n("Hours used", "1.5"), n("Price per kWh", "0.30")], result: "Cost", suffix: "", out: Kind::Currency,
        formula: "W ÷ 1000 × hours × price",
        compute: |v| Ok(&v[0] / int(1000) * &v[1] * &v[2]),
    },
    Calc {
        name: "Hourly to yearly pay", symbol: "salary", keywords: "salary wage hourly yearly annual income pay", group: DAY,
        inputs: &[n("Hourly rate", "25"), n("Hours per week", "40")], result: "Per year (52 weeks)", suffix: "", out: Kind::Currency,
        formula: "Rate × hours × 52  (÷ 12 for a month)",
        compute: |v| Ok(&v[0] * &v[1] * int(52)),
    },
    Calc {
        name: "Download time", symbol: "GB÷Mbps", keywords: "download upload transfer time speed internet file size", group: DAY,
        inputs: &[n("Size (GB)", "50"), n("Speed (Mbit/s)", "100")], result: "Time", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "GB × 8000 ÷ Mbit/s (ideal line speed)",
        compute: |v| positive(&v[1], "Speed must be above zero").map(|_| &v[0] * int(8000) / &v[1]),
    },
    Calc {
        name: "Body mass index", symbol: "BMI", keywords: "bmi body mass index weight height health", group: DAY,
        inputs: &[n("Weight (kg)", "70"), n("Height (cm)", "175")], result: "BMI", suffix: " kg/m²", out: Kind::Linear,
        formula: "kg ÷ m²  (18.5 – 25 is the WHO normal range)",
        compute: |v| {
            positive(&v[1], "Height must be above zero")?;
            let m = &v[1] / int(100);
            Ok(&v[0] / (&m * &m))
        },
    },
    Calc {
        name: "Days between dates", symbol: "Δdays", keywords: "days between dates difference countdown calendar date", group: DAY,
        inputs: &[date("From date", "today"), date("To date", "2026-12-25")], result: "Days", suffix: " days", out: Kind::Linear,
        formula: "(To − From) ÷ 86400 s, in UTC",
        compute: |v| Ok((&v[1] - &v[0]) / int(86400)),
    },
    Calc {
        name: "Add days to a date", symbol: "date+n", keywords: "add days date plus future deadline calendar", group: DAY,
        inputs: &[date("Date", "today"), n("Days (negative to go back)", "90")], result: "Date", suffix: "", out: Kind::Date(DateFmt::Readable),
        formula: "Date + days × 86400 s",
        compute: |v| Ok(&v[0] + &v[1] * int(86400)),
    },
    Calc {
        name: "Age", symbol: "age", keywords: "age birthday years old born calendar date", group: DAY,
        inputs: &[date("Birth date", "1990-01-01")], result: "Age", suffix: " years", out: Kind::Linear,
        formula: "Full years from the date until today",
        compute: |v| age(&v[0]),
    },
    // ── Health ──
    Calc {
        name: "Calories at rest (BMR), men", symbol: "BMR ♂", keywords: "bmr basal metabolic rate calories kcal diet men male mifflin", group: HEALTH,
        inputs: &[n("Weight (kg)", "80"), n("Height (cm)", "180"), n("Age (years)", "30")], result: "Calories", suffix: " kcal/day", out: Kind::Linear,
        formula: "10·kg + 6.25·cm − 5·age + 5  (Mifflin–St Jeor)",
        compute: |v| Ok(int(10) * &v[0] + R::new(25.into(), 4.into()) * &v[1] - int(5) * &v[2] + int(5)),
    },
    Calc {
        name: "Calories at rest (BMR), women", symbol: "BMR ♀", keywords: "bmr basal metabolic rate calories kcal diet women female mifflin", group: HEALTH,
        inputs: &[n("Weight (kg)", "65"), n("Height (cm)", "165"), n("Age (years)", "30")], result: "Calories", suffix: " kcal/day", out: Kind::Linear,
        formula: "10·kg + 6.25·cm − 5·age − 161  (Mifflin–St Jeor)",
        compute: |v| Ok(int(10) * &v[0] + R::new(25.into(), 4.into()) * &v[1] - int(5) * &v[2] - int(161)),
    },
    Calc {
        name: "Daily calories (TDEE)", symbol: "TDEE", keywords: "tdee total daily energy expenditure calories maintenance diet activity", group: HEALTH,
        inputs: &[n("Calories at rest (BMR)", "1750"), n("Activity: 1.2 desk … 1.55 active … 1.9 athlete", "1.375")], result: "Calories", suffix: " kcal/day", out: Kind::Linear,
        formula: "BMR × activity factor",
        compute: |v| {
            if v[1] < R::new(1.into(), 1.into()) || v[1] > R::new(25.into(), 10.into()) {
                return Err("Activity factor is between 1.0 and 2.5");
            }
            Ok(&v[0] * &v[1])
        },
    },
    Calc {
        name: "Ideal weight, men", symbol: "IBW ♂", keywords: "ideal body weight men male devine healthy", group: HEALTH,
        inputs: &[n("Height (cm)", "180")], result: "Weight", suffix: " kg", out: Kind::Linear,
        formula: "50 + 0.9055 × (cm − 152.4)  (Devine)",
        compute: |v| Ok(int(50) + num::parse_decimal("0.9055").unwrap_or_default() * (&v[0] - num::parse_decimal("152.4").unwrap_or_default())),
    },
    Calc {
        name: "Ideal weight, women", symbol: "IBW ♀", keywords: "ideal body weight women female devine healthy", group: HEALTH,
        inputs: &[n("Height (cm)", "165")], result: "Weight", suffix: " kg", out: Kind::Linear,
        formula: "45.5 + 0.9055 × (cm − 152.4)  (Devine)",
        compute: |v| Ok(num::parse_decimal("45.5").unwrap_or_default() + num::parse_decimal("0.9055").unwrap_or_default() * (&v[0] - num::parse_decimal("152.4").unwrap_or_default())),
    },
    Calc {
        name: "Body fat, men (US Navy)", symbol: "fat% ♂", keywords: "body fat percentage men male navy tape measure", group: HEALTH,
        inputs: &[n("Waist (cm)", "85"), n("Neck (cm)", "38"), n("Height (cm)", "180")], result: "Body fat", suffix: "%", out: Kind::Linear,
        formula: "495 ÷ (1.0324 − 0.19077·log(waist − neck) + 0.15456·log(height)) − 450",
        compute: |v| {
            let (w, ne, h) = (fl(&v[0])?, fl(&v[1])?, fl(&v[2])?);
            if w <= ne || h <= 0.0 {
                return Err("Waist must be larger than neck");
            }
            from_fl(495.0 / (1.0324 - 0.19077 * (w - ne).log10() + 0.15456 * h.log10()) - 450.0)
        },
    },
    Calc {
        name: "Body fat, women (US Navy)", symbol: "fat% ♀", keywords: "body fat percentage women female navy tape measure", group: HEALTH,
        inputs: &[n("Waist (cm)", "75"), n("Hips (cm)", "100"), n("Neck (cm)", "33"), n("Height (cm)", "165")], result: "Body fat", suffix: "%", out: Kind::Linear,
        formula: "495 ÷ (1.29579 − 0.35004·log(waist + hips − neck) + 0.221·log(height)) − 450",
        compute: |v| {
            let (w, hip, ne, h) = (fl(&v[0])?, fl(&v[1])?, fl(&v[2])?, fl(&v[3])?);
            if w + hip <= ne || h <= 0.0 {
                return Err("Waist + hips must be larger than neck");
            }
            from_fl(495.0 / (1.29579 - 0.35004 * (w + hip - ne).log10() + 0.221 * h.log10()) - 450.0)
        },
    },
    Calc {
        name: "Maximum heart rate", symbol: "HRmax", keywords: "max heart rate pulse training zones cardio age tanaka", group: HEALTH,
        inputs: &[n("Age (years)", "35")], result: "Max heart rate", suffix: " bpm", out: Kind::Linear,
        formula: "208 − 0.7 × age  (Tanaka)",
        compute: |v| Ok(int(208) - R::new(7.into(), 10.into()) * &v[0]),
    },
    Calc {
        name: "Daily water intake", symbol: "water", keywords: "water drink hydration liters per day", group: HEALTH,
        inputs: &[n("Weight (kg)", "70")], result: "Water", suffix: " L/day", out: Kind::Linear,
        formula: "33 ml per kg of body weight (a common guideline)",
        compute: |v| Ok(&v[0] * R::new(33.into(), 1000.into())),
    },
    Calc {
        name: "Pregnancy due date", symbol: "due", keywords: "due date pregnancy birth baby lmp last period naegele", group: HEALTH,
        inputs: &[date("First day of last period", "today"), n("Cycle length (days)", "28")], result: "Due date", suffix: "", out: Kind::Date(DateFmt::Readable),
        formula: "Last period + 280 days + (cycle − 28) days",
        compute: |v| Ok(&v[0] + (int(252) + &v[1]) * int(86400)),
    },
    Calc {
        name: "Ovulation date", symbol: "ovul.", keywords: "ovulation fertile window cycle period", group: HEALTH,
        inputs: &[date("First day of last period", "today"), n("Cycle length (days)", "28")], result: "Ovulation", suffix: "", out: Kind::Date(DateFmt::Readable),
        formula: "Last period + cycle length − 14 days",
        compute: |v| Ok(&v[0] + (&v[1] - int(14)) * int(86400)),
    },
    Calc {
        name: "Weeks pregnant", symbol: "weeks", keywords: "pregnancy week gestational age lmp", group: HEALTH,
        inputs: &[date("First day of last period", "2026-06-01")], result: "Pregnant for", suffix: " weeks", out: Kind::Linear,
        formula: "(Today − last period) ÷ 7 days",
        compute: |v| Ok((time::today() - &v[0]) / int(7 * 86400)),
    },
    // ── Photography ──
    Calc {
        name: "Exposure value (EV)", symbol: "EV", keywords: "exposure value ev aperture shutter iso light meter photography", group: PHOTO,
        inputs: &[n("Aperture (f-number)", "8"), n("Shutter (seconds, e.g. 1/125)", "1/125"), n("ISO", "100")], result: "EV at ISO 100", suffix: "", out: Kind::Linear,
        formula: "log₂(N² ÷ t) − log₂(ISO ÷ 100)",
        compute: |v| {
            let (f, t, iso) = (fl(&v[0])?, fl(&v[1])?, fl(&v[2])?);
            if f <= 0.0 || t <= 0.0 || iso <= 0.0 {
                return Err("Aperture, shutter and ISO must be above zero");
            }
            from_fl((f * f / t).log2() - (iso / 100.0).log2())
        },
    },
    Calc {
        name: "Equivalent shutter speed", symbol: "t₂", keywords: "equivalent exposure shutter speed aperture change iso photography", group: PHOTO,
        inputs: &[n("Aperture now (f-number)", "8"), n("Shutter now (s)", "1/125"), n("New aperture (f-number)", "4"), n("ISO change (new ÷ old)", "1")], result: "New shutter", suffix: " s", out: Kind::Linear,
        formula: "t × (N₂ ÷ N₁)² ÷ ISO change",
        compute: |v| {
            positive(&v[0], "Aperture must be above zero")?;
            positive(&v[3], "ISO change must be above zero")?;
            let r = &v[2] / &v[0];
            Ok(&v[1] * &r * &r / &v[3])
        },
    },
    Calc {
        name: "ND filter shutter speed", symbol: "ND t", keywords: "nd filter neutral density long exposure shutter stops photography", group: PHOTO,
        inputs: &[n("Shutter without filter (s)", "1/60"), n("Filter strength (stops)", "10")], result: "Shutter with filter", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "t × 2^stops",
        compute: |v| power(&int(2), &v[1]).map(|p| &v[0] * p),
    },
    Calc {
        name: "Equivalent focal length", symbol: "f×crop", keywords: "crop factor equivalent focal length full frame 35mm aps-c micro four thirds", group: PHOTO,
        inputs: &[n("Focal length (mm)", "50"), n("Crop factor (APS-C 1.5, MFT 2)", "1.5")], result: "Full-frame equivalent", suffix: " mm", out: Kind::Linear,
        formula: "focal length × crop factor",
        compute: |v| Ok(&v[0] * &v[1]),
    },
    Calc {
        name: "Field of view", symbol: "FOV", keywords: "angle of view field of view lens focal length sensor photography", group: PHOTO,
        inputs: &[n("Focal length (mm)", "50"), n("Sensor width (mm, full frame 36)", "36")], result: "Horizontal view", suffix: "°", out: Kind::Linear,
        formula: "2 × atan(sensor width ÷ (2 × focal length))",
        compute: |v| {
            let (f, w) = (fl(&v[0])?, fl(&v[1])?);
            if f <= 0.0 || w <= 0.0 {
                return Err("Focal length and sensor width must be above zero");
            }
            from_fl((w / (2.0 * f)).atan().to_degrees() * 2.0)
        },
    },
    Calc {
        name: "Hyperfocal distance", symbol: "H", keywords: "hyperfocal distance depth of field focus infinity landscape photography", group: PHOTO,
        inputs: &[n("Focal length (mm)", "24"), n("Aperture (f-number)", "8"), n("Circle of confusion (mm, full frame 0.03)", "0.03")], result: "Hyperfocal distance", suffix: " m", out: Kind::Linear,
        formula: "f² ÷ (N × c) + f",
        compute: |v| {
            positive(&v[1], "Aperture must be above zero")?;
            positive(&v[2], "Circle of confusion must be above zero")?;
            Ok((&v[0] * &v[0] / (&v[1] * &v[2]) + &v[0]) / int(1000))
        },
    },
    Calc {
        name: "Depth of field", symbol: "DoF", keywords: "depth of field dof sharp focus distance aperture photography", group: PHOTO,
        inputs: &[n("Focal length (mm)", "50"), n("Aperture (f-number)", "2.8"), n("Subject distance (m)", "3"), n("Circle of confusion (mm)", "0.03")], result: "In focus", suffix: " m", out: Kind::Linear,
        formula: "far − near, with H = f²/(N·c) + f",
        compute: |v| {
            let (f, n, s, c) = (fl(&v[0])?, fl(&v[1])?, fl(&v[2])? * 1000.0, fl(&v[3])?);
            if f <= 0.0 || n <= 0.0 || c <= 0.0 || s <= f {
                return Err("Values must be above zero and the subject farther than the focal length");
            }
            let h = f * f / (n * c) + f;
            if s >= h {
                return Err("Beyond the hyperfocal distance: sharp from H/2 to infinity");
            }
            let near = s * (h - f) / (h + s - 2.0 * f);
            let far = s * (h - f) / (h - s);
            from_fl((far - near) / 1000.0)
        },
    },
    Calc {
        name: "Print size from pixels", symbol: "px÷dpi", keywords: "print size pixels dpi ppi inches photo resolution", group: PHOTO,
        inputs: &[n("Pixels (one side)", "6000"), n("Print resolution (dpi)", "300")], result: "Print size", suffix: " in", out: Kind::Linear,
        formula: "pixels ÷ dpi  (× 2.54 for cm)",
        compute: |v| nonzero(&v[1], "Resolution can't be zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Megapixels", symbol: "MP", keywords: "megapixels resolution camera sensor pixels width height", group: PHOTO,
        inputs: &[n("Width (px)", "6000"), n("Height (px)", "4000")], result: "Resolution", suffix: " MP", out: Kind::Linear,
        formula: "width × height ÷ 1,000,000",
        compute: |v| Ok(&v[0] * &v[1] / int(1_000_000)),
    },
    // ── Music & audio ──
    Calc {
        name: "Tempo after pitch shift", symbol: "BPM±st", keywords: "tempo bpm pitch shift semitones transpose speed resample vinyl dj", group: MUSIC,
        inputs: &[n("Tempo (BPM)", "120"), n("Pitch shift (semitones)", "2")], result: "New tempo", suffix: " BPM", out: Kind::Linear,
        formula: "BPM × 2^(semitones ÷ 12)  (speed and pitch change together)",
        compute: |v| from_fl(fl(&v[0])? * 2f64.powf(fl(&v[1])? / 12.0)),
    },
    Calc {
        name: "Semitones between tempos", symbol: "st", keywords: "tempo change pitch semitones bpm dj mix key shift", group: MUSIC,
        inputs: &[n("From (BPM)", "120"), n("To (BPM)", "128")], result: "Pitch shift", suffix: " semitones", out: Kind::Linear,
        formula: "12 × log₂(to ÷ from)",
        compute: |v| {
            positive(&v[0], "Tempo must be above zero")?;
            positive(&v[1], "Tempo must be above zero")?;
            from_fl(12.0 * (fl(&v[1])? / fl(&v[0])?).log2())
        },
    },
    Calc {
        name: "Time-stretch percentage", symbol: "stretch%", keywords: "time stretch percentage tempo change bpm speed warp", group: MUSIC,
        inputs: &[n("From (BPM)", "120"), n("To (BPM)", "128")], result: "Speed", suffix: "%", out: Kind::Linear,
        formula: "to ÷ from × 100",
        compute: |v| nonzero(&v[0], "Tempo can't be zero").map(|_| &v[1] / &v[0] * int(100)),
    },
    Calc {
        name: "Song length", symbol: "bars→time", keywords: "song length duration bars measures beats bpm arrangement", group: MUSIC,
        inputs: &[n("Bars", "64"), n("Beats per bar", "4"), n("Tempo (BPM)", "120")], result: "Length", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "bars × beats per bar × 60 ÷ BPM",
        compute: |v| nonzero(&v[2], "Tempo can't be zero").map(|_| &v[0] * &v[1] * int(60) / &v[2]),
    },
    Calc {
        name: "Bars in a length of time", symbol: "time→bars", keywords: "bars measures count duration bpm", group: MUSIC,
        inputs: &[n("Minutes", "3"), n("Beats per bar", "4"), n("Tempo (BPM)", "120")], result: "Bars", suffix: " bars", out: Kind::Linear,
        formula: "minutes × BPM ÷ beats per bar",
        compute: |v| nonzero(&v[1], "Beats per bar can't be zero").map(|_| &v[0] * &v[2] / &v[1]),
    },
    Calc {
        name: "Uncompressed audio size", symbol: "WAV MB", keywords: "wav aiff pcm file size sample rate bit depth channels audio storage", group: MUSIC,
        inputs: &[n("Sample rate (Hz)", "48000"), n("Bit depth", "24"), n("Channels", "2"), n("Length (minutes)", "3")], result: "File size", suffix: " MB", out: Kind::Linear,
        formula: "rate × bits ÷ 8 × channels × seconds ÷ 1,000,000",
        compute: |v| Ok(&v[0] * &v[1] / int(8) * &v[2] * &v[3] * int(60) / int(1_000_000)),
    },
    Calc {
        name: "File size from bitrate", symbol: "kbps→MB", keywords: "mp3 aac opus bitrate file size kbps audio video stream", group: MUSIC,
        inputs: &[n("Bitrate (kbit/s)", "320"), n("Length (minutes)", "4")], result: "File size", suffix: " MB", out: Kind::Linear,
        formula: "kbit/s × 1000 ÷ 8 × seconds ÷ 1,000,000",
        compute: |v| Ok(&v[0] * &v[1] * int(60) / int(8000)),
    },
    Calc {
        name: "Bitrate from file size", symbol: "MB→kbps", keywords: "bitrate kbps file size audio video encode target", group: MUSIC,
        inputs: &[n("File size (MB)", "9.6"), n("Length (minutes)", "4")], result: "Bitrate", suffix: " kbit/s", out: Kind::Linear,
        formula: "MB × 8000 ÷ seconds",
        compute: |v| nonzero(&v[1], "Length can't be zero").map(|_| &v[0] * int(8000) / (&v[1] * int(60))),
    },
    Calc {
        name: "Fret position", symbol: "fret", keywords: "guitar bass fret position scale length luthier string", group: MUSIC,
        inputs: &[n("Scale length (mm, Fender 648)", "648"), n("Fret number", "12")], result: "Distance from nut", suffix: " mm", out: Kind::Linear,
        formula: "L − L ÷ 2^(fret ÷ 12)",
        compute: |v| {
            let l = fl(&v[0])?;
            from_fl(l - l / 2f64.powf(fl(&v[1])? / 12.0))
        },
    },
    Calc {
        name: "Room resonance (axial mode)", symbol: "f mode", keywords: "room mode resonance acoustics standing wave bass studio treatment", group: MUSIC,
        inputs: &[n("Room dimension (m)", "4"), n("Mode number", "1")], result: "Frequency", suffix: " Hz", out: Kind::Linear,
        formula: "n × 343 ÷ (2 × length)",
        compute: |v| {
            positive(&v[0], "Dimension must be above zero")?;
            Ok(&v[1] * int(343) / (int(2) * &v[0]))
        },
    },
    // ── More percentages ──
    Calc {
        name: "Percent error", symbol: "% err", keywords: "percent error accuracy measured actual experimental theoretical", group: PCT,
        inputs: &[n("Measured", "9.6"), n("Actual", "9.81")], result: "Error", suffix: "%", out: Kind::Linear,
        formula: "|measured − actual| ÷ |actual| × 100",
        compute: |v| nonzero(&v[1], "Actual value can't be zero").map(|_| (&v[0] - &v[1]).abs() / v[1].abs() * int(100)),
    },
    // ── More math ──
    Calc {
        name: "Distance between two points", symbol: "dist", keywords: "distance points coordinates geometry x y length", group: MATH,
        inputs: &[n("x₁", "0"), n("y₁", "0"), n("x₂", "3"), n("y₂", "4")], result: "Distance", suffix: "", out: Kind::Linear,
        formula: "√((x₂ − x₁)² + (y₂ − y₁)²)",
        compute: |v| {
            let (dx, dy) = (fl(&(&v[2] - &v[0]))?, fl(&(&v[3] - &v[1]))?);
            from_fl(dx.hypot(dy))
        },
    },
    Calc {
        name: "Slope of a line", symbol: "slope", keywords: "slope gradient line rise over run geometry", group: MATH,
        inputs: &[n("x₁", "0"), n("y₁", "0"), n("x₂", "4"), n("y₂", "2")], result: "Slope", suffix: "", out: Kind::Linear,
        formula: "(y₂ − y₁) ÷ (x₂ − x₁)",
        compute: |v| {
            let run = &v[2] - &v[0];
            nonzero(&run, "The line is vertical (x₁ = x₂)")?;
            Ok((&v[3] - &v[1]) / run)
        },
    },
    Calc {
        name: "Sphere volume", symbol: "V sphere", keywords: "sphere ball volume radius geometry", group: MATH,
        inputs: &[n("Radius", "1")], result: "Volume", suffix: "", out: Kind::Linear,
        formula: "4⁄3 × π × r³",
        compute: |v| Ok(int(4) / int(3) * pi() * &v[0] * &v[0] * &v[0]),
    },
    Calc {
        name: "Cylinder volume", symbol: "V cyl", keywords: "cylinder tank pipe can volume radius height geometry", group: MATH,
        inputs: &[n("Radius", "1"), n("Height", "2")], result: "Volume", suffix: "", out: Kind::Linear,
        formula: "π × r² × h",
        compute: |v| Ok(pi() * &v[0] * &v[0] * &v[1]),
    },
    Calc {
        name: "Cone volume", symbol: "V cone", keywords: "cone volume radius height geometry funnel", group: MATH,
        inputs: &[n("Radius", "1"), n("Height", "3")], result: "Volume", suffix: "", out: Kind::Linear,
        formula: "π × r² × h ÷ 3",
        compute: |v| Ok(pi() * &v[0] * &v[0] * &v[1] / int(3)),
    },
    Calc {
        name: "Triangle area from three sides", symbol: "Heron", keywords: "triangle area heron sides geometry", group: MATH,
        inputs: &[n("Side a", "3"), n("Side b", "4"), n("Side c", "5")], result: "Area", suffix: "", out: Kind::Linear,
        formula: "√(s(s−a)(s−b)(s−c)),  s = (a + b + c) ÷ 2",
        compute: |v| {
            let (a, b, c) = (fl(&v[0])?, fl(&v[1])?, fl(&v[2])?);
            let s = (a + b + c) / 2.0;
            let q = s * (s - a) * (s - b) * (s - c);
            if a <= 0.0 || b <= 0.0 || c <= 0.0 || q < 0.0 {
                return Err("Those sides don't make a triangle");
            }
            from_fl(q.sqrt())
        },
    },
    Calc {
        name: "Triangle area from base and height", symbol: "½bh", keywords: "triangle area base height geometry", group: MATH,
        inputs: &[n("Base", "6"), n("Height", "4")], result: "Area", suffix: "", out: Kind::Linear,
        formula: "base × height ÷ 2",
        compute: |v| Ok(&v[0] * &v[1] / int(2)),
    },
    Calc {
        name: "Sine", symbol: "sin", keywords: "sin sine trigonometry angle degrees", group: MATH,
        inputs: &[n("Angle (°)", "30")], result: "sin", suffix: "", out: Kind::Linear,
        formula: "sin(angle)",
        compute: |v| from_fl(deg(&v[0])?.sin()),
    },
    Calc {
        name: "Cosine", symbol: "cos", keywords: "cos cosine trigonometry angle degrees", group: MATH,
        inputs: &[n("Angle (°)", "60")], result: "cos", suffix: "", out: Kind::Linear,
        formula: "cos(angle)",
        compute: |v| from_fl(deg(&v[0])?.cos()),
    },
    Calc {
        name: "Tangent", symbol: "tan", keywords: "tan tangent trigonometry angle degrees slope", group: MATH,
        inputs: &[n("Angle (°)", "45")], result: "tan", suffix: "", out: Kind::Linear,
        formula: "tan(angle)",
        compute: |v| {
            let a = fl(&v[0])?;
            if (a - 90.0).rem_euclid(180.0) == 0.0 {
                return Err("Tangent is undefined at 90° + k·180°");
            }
            from_fl(a.to_radians().tan())
        },
    },
    Calc {
        name: "Angle from two sides", symbol: "atan", keywords: "angle arctan atan inverse tangent opposite adjacent slope degrees trigonometry roof pitch", group: MATH,
        inputs: &[n("Opposite (rise)", "1"), n("Adjacent (run)", "1")], result: "Angle", suffix: "°", out: Kind::Linear,
        formula: "atan2(opposite, adjacent)",
        compute: |v| from_fl(fl(&v[0])?.atan2(fl(&v[1])?).to_degrees()),
    },
    Calc {
        name: "Fibonacci number", symbol: "F(n)", keywords: "fibonacci sequence number math", group: MATH,
        inputs: &[n("n", "50")], result: "F(n)", suffix: "", out: Kind::Linear,
        formula: "F(n) = F(n−1) + F(n−2),  F(0) = 0, F(1) = 1",
        compute: |v| {
            let n = whole(&v[0], 20_000, "Use a whole number from 0 to 20000")?;
            let (mut a, mut b) = (BigInt::zero(), BigInt::one());
            for _ in 0..n {
                let next = &a + &b;
                a = std::mem::replace(&mut b, next);
            }
            Ok(R::from_integer(a))
        },
    },
    // ── More electronics ──
    Calc {
        name: "Resistors in series", symbol: "R ser", keywords: "series resistors total resistance sum electronics", group: ELEC,
        inputs: &[n("R₁ (Ω)", "1000"), n("R₂ (Ω)", "2200"), n("R₃ (Ω)", "0"), n("R₄ (Ω)", "0")], result: "Total", suffix: " Ω", out: Kind::Linear,
        formula: "R₁ + R₂ + R₃ + R₄",
        compute: |v| Ok(v.iter().sum()),
    },
    Calc {
        name: "Capacitors in series", symbol: "C ser", keywords: "series capacitors total capacitance electronics", group: ELEC,
        inputs: &[n("C₁ (µF)", "10"), n("C₂ (µF)", "10")], result: "Total", suffix: " µF", out: Kind::Linear,
        formula: "C₁ × C₂ ÷ (C₁ + C₂)",
        compute: |v| {
            let sum = &v[0] + &v[1];
            nonzero(&sum, "Total can't be zero")?;
            Ok(&v[0] * &v[1] / sum)
        },
    },
    Calc {
        name: "Capacitors in parallel", symbol: "C par", keywords: "parallel capacitors total capacitance electronics", group: ELEC,
        inputs: &[n("C₁ (µF)", "10"), n("C₂ (µF)", "4.7"), n("C₃ (µF)", "0"), n("C₄ (µF)", "0")], result: "Total", suffix: " µF", out: Kind::Linear,
        formula: "C₁ + C₂ + C₃ + C₄",
        compute: |v| Ok(v.iter().sum()),
    },
    Calc {
        name: "Nearest standard resistor value", symbol: "E12/E24", keywords: "e series e12 e24 e96 standard resistor capacitor value preferred nearest electronics", group: ELEC,
        inputs: &[n("Value", "5000"), n("Series (6, 12, 24, 48, 96, 192)", "24")], result: "Nearest standard value", suffix: "", out: Kind::Linear,
        formula: "Closest value of the E-series, by ratio",
        compute: |v| {
            let series = whole(&v[1], 192, "Series is 6, 12, 24, 48, 96 or 192")?;
            super::parts::nearest_e(&v[0], series)
        },
    },
    Calc {
        name: "Inductive reactance", symbol: "X_L", keywords: "inductive reactance inductor impedance frequency ac electronics", group: ELEC,
        inputs: &[n("Frequency (Hz)", "1000"), n("Inductance (mH)", "10")], result: "Reactance", suffix: " Ω", out: Kind::Linear,
        formula: "2π × f × L",
        compute: |v| Ok(int(2) * pi() * &v[0] * &v[1] / int(1000)),
    },
    Calc {
        name: "Capacitive reactance", symbol: "X_C", keywords: "capacitive reactance capacitor impedance frequency ac electronics", group: ELEC,
        inputs: &[n("Frequency (Hz)", "1000"), n("Capacitance (µF)", "1")], result: "Reactance", suffix: " Ω", out: Kind::Linear,
        formula: "1 ÷ (2π × f × C)",
        compute: |v| {
            let d = int(2) * pi() * &v[0] * &v[1] / int(1_000_000);
            nonzero(&d, "Frequency and capacitance must be above zero")?;
            Ok(R::one() / d)
        },
    },
    Calc {
        name: "RC filter cutoff frequency", symbol: "f_c RC", keywords: "rc low pass high pass filter cutoff corner frequency -3db electronics audio", group: ELEC,
        inputs: &[n("Resistance (Ω)", "1000"), n("Capacitance (nF)", "100")], result: "Cutoff", suffix: " Hz", out: Kind::Linear,
        formula: "1 ÷ (2π × R × C)",
        compute: |v| {
            let d = int(2) * pi() * &v[0] * &v[1] / int(1_000_000_000);
            nonzero(&d, "R and C must be above zero")?;
            Ok(R::one() / d)
        },
    },
    Calc {
        name: "RL filter cutoff frequency", symbol: "f_c RL", keywords: "rl filter cutoff corner frequency inductor electronics", group: ELEC,
        inputs: &[n("Resistance (Ω)", "100"), n("Inductance (mH)", "10")], result: "Cutoff", suffix: " Hz", out: Kind::Linear,
        formula: "R ÷ (2π × L)",
        compute: |v| {
            nonzero(&v[1], "Inductance must be above zero")?;
            Ok(&v[0] / (int(2) * pi() * &v[1] / int(1000)))
        },
    },
    Calc {
        name: "555 timer frequency (astable)", symbol: "555 f", keywords: "555 timer astable oscillator frequency blinker ne555 electronics", group: ELEC,
        inputs: &[n("R1 (Ω)", "1000"), n("R2 (Ω)", "10000"), n("C (µF)", "10")], result: "Frequency", suffix: " Hz", out: Kind::Linear,
        formula: "1 ÷ (ln 2 × (R1 + 2·R2) × C)",
        compute: |v| {
            let rc = fl(&((&v[0] + int(2) * &v[1]) * &v[2]))? * 1e-6;
            if rc <= 0.0 {
                return Err("Resistors and capacitor must be above zero");
            }
            from_fl(1.0 / (std::f64::consts::LN_2 * rc))
        },
    },
    Calc {
        name: "555 timer duty cycle (astable)", symbol: "555 duty", keywords: "555 timer astable duty cycle high time electronics", group: ELEC,
        inputs: &[n("R1 (Ω)", "1000"), n("R2 (Ω)", "10000")], result: "Duty cycle", suffix: "%", out: Kind::Linear,
        formula: "(R1 + R2) ÷ (R1 + 2·R2) × 100",
        compute: |v| {
            let d = &v[0] + int(2) * &v[1];
            nonzero(&d, "Resistors must be above zero")?;
            Ok((&v[0] + &v[1]) / d * int(100))
        },
    },
    Calc {
        name: "555 timer pulse length (monostable)", symbol: "555 t", keywords: "555 timer monostable one shot pulse delay electronics", group: ELEC,
        inputs: &[n("R (Ω)", "100000"), n("C (µF)", "10")], result: "Pulse length", suffix: " s", out: Kind::Linear,
        formula: "1.1 × R × C",
        compute: |v| Ok(int(11) / int(10) * &v[0] * &v[1] / int(1_000_000)),
    },
    Calc {
        name: "Op-amp gain (non-inverting)", symbol: "1+Rf/Rg", keywords: "op amp opamp operational amplifier gain non inverting feedback electronics", group: ELEC,
        inputs: &[n("Feedback Rf (Ω)", "9000"), n("Ground Rg (Ω)", "1000")], result: "Gain", suffix: "×", out: Kind::Linear,
        formula: "1 + Rf ÷ Rg",
        compute: |v| nonzero(&v[1], "Rg can't be zero").map(|_| R::one() + &v[0] / &v[1]),
    },
    Calc {
        name: "Op-amp gain (inverting)", symbol: "−Rf/Rin", keywords: "op amp opamp operational amplifier gain inverting feedback electronics", group: ELEC,
        inputs: &[n("Feedback Rf (Ω)", "10000"), n("Input Rin (Ω)", "1000")], result: "Gain", suffix: "×", out: Kind::Linear,
        formula: "−Rf ÷ Rin",
        compute: |v| nonzero(&v[1], "Rin can't be zero").map(|_| -(&v[0] / &v[1])),
    },
    Calc {
        name: "LM317 output voltage", symbol: "LM317", keywords: "lm317 adjustable voltage regulator output resistor divider electronics", group: ELEC,
        inputs: &[n("R1 (Ω)", "240"), n("R2 (Ω)", "720")], result: "Output", suffix: " V", out: Kind::Linear,
        formula: "1.25 × (1 + R2 ÷ R1)",
        compute: |v| nonzero(&v[0], "R1 can't be zero").map(|_| int(5) / int(4) * (R::one() + &v[1] / &v[0])),
    },
    Calc {
        name: "Wire resistance", symbol: "R wire", keywords: "wire cable resistance length cross section copper aluminium resistivity electronics", group: ELEC,
        inputs: &[n("Length (m)", "10"), n("Cross-section (mm²)", "1.5"), n("Resistivity (Ω·m, copper 1.68e-8)", "1.68e-8")], result: "Resistance", suffix: " Ω", out: Kind::Linear,
        formula: "ρ × length ÷ area",
        compute: |v| {
            positive(&v[1], "Cross-section must be above zero")?;
            Ok(&v[2] * &v[0] / (&v[1] / int(1_000_000)))
        },
    },
    Calc {
        name: "Cable voltage drop", symbol: "V drop", keywords: "cable wire voltage drop loss length current cross section copper dc electronics", group: ELEC,
        inputs: &[n("Current (A)", "10"), n("One-way length (m)", "10"), n("Cross-section (mm²)", "2.5"), n("Resistivity (Ω·m, copper 1.68e-8)", "1.68e-8")], result: "Voltage drop", suffix: " V", out: Kind::Linear,
        formula: "I × ρ × 2 × length ÷ area (there and back)",
        compute: |v| {
            positive(&v[2], "Cross-section must be above zero")?;
            Ok(&v[0] * &v[3] * int(2) * &v[1] / (&v[2] / int(1_000_000)))
        },
    },
    Calc {
        name: "Battery energy", symbol: "Wh", keywords: "battery energy watt hours mah capacity power bank flight lithium electronics", group: ELEC,
        inputs: &[n("Capacity (mAh)", "10000"), n("Voltage (V)", "3.7")], result: "Energy", suffix: " Wh", out: Kind::Linear,
        formula: "mAh × V ÷ 1000",
        compute: |v| Ok(&v[0] * &v[1] / int(1000)),
    },
    Calc {
        name: "Battery charge time", symbol: "t charge", keywords: "battery charge time charging current mah electronics", group: ELEC,
        inputs: &[n("Capacity (mAh)", "3000"), n("Charge current (mA)", "1000"), n("Efficiency (%)", "85")], result: "Time", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "capacity ÷ (current × efficiency)",
        compute: |v| {
            positive(&v[1], "Current must be above zero")?;
            positive(&v[2], "Efficiency must be above zero")?;
            Ok(&v[0] / (&v[1] * pct(&v[2])) * int(3600))
        },
    },
    Calc {
        name: "Capacitor energy", symbol: "½CV²", keywords: "capacitor energy stored joules voltage electronics", group: ELEC,
        inputs: &[n("Capacitance (µF)", "1000"), n("Voltage (V)", "25")], result: "Energy", suffix: " J", out: Kind::Linear,
        formula: "½ × C × V²",
        compute: |v| Ok(&v[0] / int(1_000_000) * &v[1] * &v[1] / int(2)),
    },
    Calc {
        name: "RMS from peak (sine)", symbol: "Vrms", keywords: "rms peak sine wave ac voltage amplitude electronics", group: ELEC,
        inputs: &[n("Peak voltage", "325")], result: "RMS", suffix: "", out: Kind::Linear,
        formula: "peak ÷ √2",
        compute: |v| from_fl(fl(&v[0])? / std::f64::consts::SQRT_2),
    },
    Calc {
        name: "Peak from RMS (sine)", symbol: "Vpk", keywords: "peak rms sine wave ac voltage mains amplitude peak to peak electronics", group: ELEC,
        inputs: &[n("RMS voltage", "230")], result: "Peak", suffix: "", out: Kind::Linear,
        formula: "RMS × √2  (peak-to-peak is twice that)",
        compute: |v| from_fl(fl(&v[0])? * std::f64::consts::SQRT_2),
    },
    Calc {
        name: "Transformer output voltage", symbol: "Ns/Np", keywords: "transformer turns ratio output voltage primary secondary windings electronics", group: ELEC,
        inputs: &[n("Input voltage", "230"), n("Primary turns", "1000"), n("Secondary turns", "52")], result: "Output", suffix: " V", out: Kind::Linear,
        formula: "Vin × Ns ÷ Np",
        compute: |v| nonzero(&v[1], "Primary turns can't be zero").map(|_| &v[0] * &v[2] / &v[1]),
    },
    Calc {
        name: "PCB trace width", symbol: "trace", keywords: "pcb trace width current copper oz ipc-2221 track printed circuit board electronics", group: ELEC,
        inputs: &[n("Current (A)", "1"), n("Temperature rise (°C)", "10"), n("Copper weight (oz/ft²)", "1")], result: "Width (outer layer)", suffix: " mm", out: Kind::Linear,
        formula: "IPC-2221: area = (I ÷ (0.048 × ΔT^0.44))^(1 ÷ 0.725) mil², width = area ÷ (oz × 1.378 mil)",
        compute: |v| {
            let (i, dt, oz) = (fl(&v[0])?, fl(&v[1])?, fl(&v[2])?);
            if i <= 0.0 || dt <= 0.0 || oz <= 0.0 {
                return Err("All values must be above zero");
            }
            let area = (i / (0.048 * dt.powf(0.44))).powf(1.0 / 0.725);
            from_fl(area / (oz * 1.378) * 0.0254)
        },
    },
    Calc {
        name: "PWM average voltage", symbol: "PWM", keywords: "pwm duty cycle average voltage motor led dimming electronics", group: ELEC,
        inputs: &[n("Supply voltage", "12"), n("Duty cycle (%)", "25")], result: "Average", suffix: " V", out: Kind::Linear,
        formula: "V × duty ÷ 100",
        compute: |v| Ok(&v[0] * pct(&v[1])),
    },
    Calc {
        name: "AC power with power factor", symbol: "V·I·PF", keywords: "ac power real power factor watts va apparent single phase electronics", group: ELEC,
        inputs: &[n("Voltage (V)", "230"), n("Current (A)", "5"), n("Power factor", "0.9")], result: "Real power", suffix: " W", out: Kind::Linear,
        formula: "V × I × PF",
        compute: |v| Ok(&v[0] * &v[1] * &v[2]),
    },
    Calc {
        name: "Three-phase power", symbol: "3φ kW", keywords: "three phase 3 phase power kw line voltage current power factor motor electronics", group: ELEC,
        inputs: &[n("Line voltage (V)", "400"), n("Current (A)", "16"), n("Power factor", "0.9")], result: "Power", suffix: " kW", out: Kind::Linear,
        formula: "√3 × V × I × PF ÷ 1000",
        compute: |v| from_fl(3f64.sqrt() * fl(&(&v[0] * &v[1] * &v[2]))? / 1000.0),
    },
    Calc {
        name: "Quarter-wave antenna length", symbol: "λ/4", keywords: "antenna quarter wave wavelength length frequency radio ham whip rf electronics", group: ELEC,
        inputs: &[n("Frequency (MHz)", "433"), n("Velocity factor", "0.95")], result: "Length", suffix: " cm", out: Kind::Linear,
        formula: "c ÷ f ÷ 4 × velocity factor",
        compute: |v| {
            positive(&v[0], "Frequency must be above zero")?;
            Ok(super::expr("c") / (&v[0] * int(1_000_000)) / int(4) * &v[1] * int(100))
        },
    },
    Calc {
        name: "Solar panel daily energy", symbol: "PV Wh", keywords: "solar panel photovoltaic pv daily energy sun hours output watt", group: ELEC,
        inputs: &[n("Panel power (W)", "400"), n("Peak sun hours", "4"), n("System efficiency (%)", "80")], result: "Energy per day", suffix: " Wh", out: Kind::Linear,
        formula: "W × sun hours × efficiency",
        compute: |v| Ok(&v[0] * &v[1] * pct(&v[2])),
    },
    // ── Physics ──
    Calc {
        name: "Kinetic energy", symbol: "½mv²", keywords: "kinetic energy mass velocity speed joules physics", group: PHYS,
        inputs: &[n("Mass (kg)", "1000"), n("Speed (m/s)", "20")], result: "Energy", suffix: " J", out: Kind::Linear,
        formula: "½ × m × v²",
        compute: |v| Ok(&v[0] * &v[1] * &v[1] / int(2)),
    },
    Calc {
        name: "Potential energy", symbol: "mgh", keywords: "potential energy gravity height mass joules physics", group: PHYS,
        inputs: &[n("Mass (kg)", "10"), n("Height (m)", "5")], result: "Energy", suffix: " J", out: Kind::Linear,
        formula: "m × g × h  (g = 9.80665 m/s²)",
        compute: |v| Ok(&v[0] * g0() * &v[1]),
    },
    Calc {
        name: "Momentum", symbol: "p = mv", keywords: "momentum mass velocity physics impulse", group: PHYS,
        inputs: &[n("Mass (kg)", "2"), n("Speed (m/s)", "3")], result: "Momentum", suffix: " kg·m/s", out: Kind::Linear,
        formula: "m × v",
        compute: |v| Ok(&v[0] * &v[1]),
    },
    Calc {
        name: "Force from mass and acceleration", symbol: "F = ma", keywords: "force newton second law mass acceleration physics", group: PHYS,
        inputs: &[n("Mass (kg)", "10"), n("Acceleration (m/s²)", "9.80665")], result: "Force", suffix: " N", out: Kind::Linear,
        formula: "m × a",
        compute: |v| Ok(&v[0] * &v[1]),
    },
    Calc {
        name: "Pressure from force and area", symbol: "F/A", keywords: "pressure force area pascal physics", group: PHYS,
        inputs: &[n("Force (N)", "100"), n("Area (m²)", "0.01")], result: "Pressure", suffix: " Pa", out: Kind::Linear,
        formula: "F ÷ A",
        compute: |v| nonzero(&v[1], "Area can't be zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Density from mass and volume", symbol: "ρ = m/V", keywords: "density mass volume physics material", group: PHYS,
        inputs: &[n("Mass (g)", "19.3"), n("Volume (cm³)", "1")], result: "Density", suffix: " g/cm³", out: Kind::Linear,
        formula: "m ÷ V",
        compute: |v| nonzero(&v[1], "Volume can't be zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Free fall time", symbol: "t fall", keywords: "free fall drop time height gravity physics", group: PHYS,
        inputs: &[n("Height (m)", "20")], result: "Time", suffix: " s", out: Kind::Linear,
        formula: "√(2h ÷ g)  (no air resistance)",
        compute: |v| {
            if v[0].is_negative() {
                return Err("Height can't be negative");
            }
            from_fl((2.0 * fl(&v[0])? / 9.80665).sqrt())
        },
    },
    Calc {
        name: "Free fall impact speed", symbol: "v fall", keywords: "free fall impact speed velocity height gravity physics", group: PHYS,
        inputs: &[n("Height (m)", "20")], result: "Speed", suffix: " m/s", out: Kind::Linear,
        formula: "√(2gh)  (no air resistance)",
        compute: |v| {
            if v[0].is_negative() {
                return Err("Height can't be negative");
            }
            from_fl((2.0 * 9.80665 * fl(&v[0])?).sqrt())
        },
    },
    Calc {
        name: "Projectile range", symbol: "range", keywords: "projectile range ballistic throw launch angle speed distance physics", group: PHYS,
        inputs: &[n("Launch speed (m/s)", "20"), n("Angle (°)", "45")], result: "Range (flat ground)", suffix: " m", out: Kind::Linear,
        formula: "v² × sin(2θ) ÷ g",
        compute: |v| from_fl(fl(&v[0])?.powi(2) * (2.0 * deg(&v[1])?).sin() / 9.80665),
    },
    Calc {
        name: "Pendulum period", symbol: "T pend", keywords: "pendulum period swing length gravity physics clock", group: PHYS,
        inputs: &[n("Length (m)", "1")], result: "Period", suffix: " s", out: Kind::Linear,
        formula: "2π × √(L ÷ g)",
        compute: |v| {
            positive(&v[0], "Length must be above zero")?;
            from_fl(std::f64::consts::TAU * (fl(&v[0])? / 9.80665).sqrt())
        },
    },
    Calc {
        name: "Centripetal force", symbol: "mv²/r", keywords: "centripetal force circular motion radius speed physics", group: PHYS,
        inputs: &[n("Mass (kg)", "1"), n("Speed (m/s)", "10"), n("Radius (m)", "5")], result: "Force", suffix: " N", out: Kind::Linear,
        formula: "m × v² ÷ r",
        compute: |v| nonzero(&v[2], "Radius can't be zero").map(|_| &v[0] * &v[1] * &v[1] / &v[2]),
    },
    Calc {
        name: "Gravitational force", symbol: "Gm₁m₂/r²", keywords: "gravity gravitational force newton masses distance physics", group: PHYS,
        inputs: &[n("Mass 1 (kg)", "5.972e24"), n("Mass 2 (kg)", "7.342e22"), n("Distance (m)", "3.844e8")], result: "Force", suffix: " N", out: Kind::Linear,
        formula: "G × m₁ × m₂ ÷ r²  (G = 6.6743e-11)",
        compute: |v| {
            positive(&v[2], "Distance must be above zero")?;
            from_fl(BIG_G * fl(&v[0])? * fl(&v[1])? / fl(&v[2])?.powi(2))
        },
    },
    Calc {
        name: "Escape velocity", symbol: "v esc", keywords: "escape velocity planet mass radius gravity space physics", group: PHYS,
        inputs: &[n("Planet mass (kg)", "5.972e24"), n("Radius (m)", "6.371e6")], result: "Escape velocity", suffix: " m/s", out: Kind::Linear,
        formula: "√(2GM ÷ r)",
        compute: |v| {
            positive(&v[0], "Mass must be above zero")?;
            positive(&v[1], "Radius must be above zero")?;
            from_fl((2.0 * BIG_G * fl(&v[0])? / fl(&v[1])?).sqrt())
        },
    },
    Calc {
        name: "Mass–energy equivalence", symbol: "E = mc²", keywords: "e=mc2 mass energy equivalence einstein relativity physics", group: PHYS,
        inputs: &[n("Mass (kg)", "0.001")], result: "Energy", suffix: " J", out: Kind::Linear,
        formula: "m × c²",
        compute: |v| Ok(&v[0] * super::expr("c^2")),
    },
    Calc {
        name: "Heat energy", symbol: "Q = mcΔT", keywords: "heat energy specific heat temperature change mass water physics thermodynamics", group: PHYS,
        inputs: &[n("Mass (kg)", "1"), n("Specific heat (J/(kg·K), water 4186)", "4186"), n("Temperature change (°C)", "80")], result: "Energy", suffix: " kJ", out: Kind::Linear,
        formula: "m × c × ΔT",
        compute: |v| Ok(&v[0] * &v[1] * &v[2] / int(1000)),
    },
    Calc {
        name: "Time to heat water", symbol: "kettle", keywords: "kettle boil water heating time power watts temperature physics", group: PHYS,
        inputs: &[n("Water (L)", "1"), n("From (°C)", "20"), n("To (°C)", "100"), n("Heater power (W)", "2000")], result: "Time (no losses)", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "4186 × liters × (to − from) ÷ watts",
        compute: |v| {
            positive(&v[3], "Power must be above zero")?;
            Ok(int(4186) * &v[0] * (&v[2] - &v[1]) / &v[3])
        },
    },
    Calc {
        name: "Radioactive decay (half-life)", symbol: "N(t)", keywords: "half life half-life decay radioactive remaining amount carbon dating physics medicine", group: PHYS,
        inputs: &[n("Starting amount", "100"), n("Half-life", "5730"), n("Time passed (same unit)", "10000")], result: "Remaining", suffix: "", out: Kind::Linear,
        formula: "N₀ × ½^(t ÷ half-life)",
        compute: |v| {
            positive(&v[1], "Half-life must be above zero")?;
            from_fl(fl(&v[0])? * 0.5f64.powf(fl(&v[2])? / fl(&v[1])?))
        },
    },
    Calc {
        name: "Wavelength from frequency", symbol: "λ = v/f", keywords: "wavelength frequency wave speed sound light physics", group: PHYS,
        inputs: &[n("Frequency (Hz)", "440"), n("Wave speed (m/s, sound 343)", "343")], result: "Wavelength", suffix: " m", out: Kind::Linear,
        formula: "v ÷ f",
        compute: |v| nonzero(&v[0], "Frequency can't be zero").map(|_| &v[1] / &v[0]),
    },
    Calc {
        name: "Speed of sound in air", symbol: "c air", keywords: "speed of sound air temperature acoustics physics", group: PHYS,
        inputs: &[n("Temperature (°C)", "20")], result: "Speed", suffix: " m/s", out: Kind::Linear,
        formula: "331.3 × √(1 + T ÷ 273.15)",
        compute: |v| {
            let t = fl(&v[0])?;
            if t <= -273.15 {
                return Err("Below absolute zero");
            }
            from_fl(331.3 * (1.0 + t / 273.15).sqrt())
        },
    },
    Calc {
        name: "Distance to lightning", symbol: "thunder", keywords: "lightning thunder distance storm seconds count sound physics", group: PHYS,
        inputs: &[n("Seconds between flash and thunder", "6")], result: "Distance", suffix: " km", out: Kind::Linear,
        formula: "seconds × 343 m/s",
        compute: |v| Ok(&v[0] * int(343) / int(1000)),
    },
    // ── Business & finance ──
    Calc {
        name: "Add VAT / sales tax", symbol: "+VAT", keywords: "vat sales tax gst add net to gross price business", group: BIZ,
        inputs: &[n("Net price", "100"), n("Tax rate (%)", "20")], result: "Price with tax", suffix: "", out: Kind::Currency,
        formula: "net × (1 + rate ÷ 100)",
        compute: |v| Ok(&v[0] * (R::one() + pct(&v[1]))),
    },
    Calc {
        name: "Remove VAT / sales tax", symbol: "−VAT", keywords: "vat sales tax gst remove gross to net price excluding business", group: BIZ,
        inputs: &[n("Price with tax", "120"), n("Tax rate (%)", "20")], result: "Net price", suffix: "", out: Kind::Currency,
        formula: "gross ÷ (1 + rate ÷ 100)",
        compute: |v| {
            let d = R::one() + pct(&v[1]);
            nonzero(&d, "Rate can't be −100%")?;
            Ok(&v[0] / d)
        },
    },
    Calc {
        name: "Tax included in a price", symbol: "VAT in", keywords: "vat sales tax gst amount included in gross price business", group: BIZ,
        inputs: &[n("Price with tax", "120"), n("Tax rate (%)", "20")], result: "Tax amount", suffix: "", out: Kind::Currency,
        formula: "gross × rate ÷ (100 + rate)",
        compute: |v| {
            let d = int(100) + &v[1];
            nonzero(&d, "Rate can't be −100%")?;
            Ok(&v[0] * &v[1] / d)
        },
    },
    Calc {
        name: "Break-even point", symbol: "BEP", keywords: "break even point units sales fixed variable cost price contribution business", group: BIZ,
        inputs: &[n("Fixed costs", "10000"), n("Price per unit", "50"), n("Variable cost per unit", "30")], result: "Units to break even", suffix: " units", out: Kind::Linear,
        formula: "fixed ÷ (price − variable cost)",
        compute: |v| {
            let margin = &v[1] - &v[2];
            positive(&margin, "Price must be above the variable cost")?;
            Ok(&v[0] / margin)
        },
    },
    Calc {
        name: "Return on investment (ROI)", symbol: "ROI", keywords: "roi return on investment profit gain business", group: BIZ,
        inputs: &[n("Final value", "1500"), n("Cost", "1000")], result: "ROI", suffix: "%", out: Kind::Linear,
        formula: "(final − cost) ÷ cost × 100",
        compute: |v| nonzero(&v[1], "Cost can't be zero").map(|_| (&v[0] - &v[1]) / &v[1] * int(100)),
    },
    Calc {
        name: "Compound annual growth rate (CAGR)", symbol: "CAGR", keywords: "cagr compound annual growth rate return investment business", group: BIZ,
        inputs: &[n("Start value", "1000"), n("End value", "2000"), n("Years", "5")], result: "CAGR", suffix: "%", out: Kind::Linear,
        formula: "((end ÷ start)^(1 ÷ years) − 1) × 100",
        compute: |v| {
            positive(&v[0], "Start value must be above zero")?;
            positive(&v[2], "Years must be above zero")?;
            let ratio = fl(&(&v[1] / &v[0]))?;
            if ratio < 0.0 {
                return Err("End value can't be negative");
            }
            from_fl((ratio.powf(1.0 / fl(&v[2])?) - 1.0) * 100.0)
        },
    },
    Calc {
        name: "Doubling time", symbol: "2× time", keywords: "doubling time rule of 72 growth interest rate years business", group: BIZ,
        inputs: &[n("Growth per year (%)", "7")], result: "Years to double", suffix: " years", out: Kind::Linear,
        formula: "ln 2 ÷ ln(1 + rate ÷ 100)  (the rule of 72 approximates this)",
        compute: |v| {
            positive(&v[0], "Growth must be above zero")?;
            from_fl(std::f64::consts::LN_2 / (1.0 + fl(&v[0])? / 100.0).ln())
        },
    },
    Calc {
        name: "Present value", symbol: "PV", keywords: "present value discount inflation purchasing power future money worth today business", group: BIZ,
        inputs: &[n("Future amount", "10000"), n("Rate per year (%)", "3"), n("Years", "10")], result: "Worth today", suffix: "", out: Kind::Currency,
        formula: "amount ÷ (1 + rate)^years",
        compute: |v| {
            let base = R::one() + pct(&v[1]);
            positive(&base, "Rate must be above −100%")?;
            Ok(&v[0] / power(&base, &v[2])?)
        },
    },
    Calc {
        name: "Savings with monthly deposits", symbol: "FV", keywords: "savings future value monthly deposit contribution investment retirement compound business", group: BIZ,
        inputs: &[n("Monthly deposit", "200"), n("Yearly return (%)", "5"), n("Years", "20"), n("Starting balance", "0")], result: "Balance", suffix: "", out: Kind::Currency,
        formula: "P × (1 + r)^n + D × ((1 + r)^n − 1) ÷ r,  r = rate ÷ 1200, n = years × 12",
        compute: |v| {
            let months = &v[2] * int(12);
            let r = &v[1] / int(1200);
            if r.is_zero() {
                return Ok(&v[3] + &v[0] * months);
            }
            let g = power(&(R::one() + &r), &months)?;
            Ok(&v[3] * &g + &v[0] * (g - R::one()) / r)
        },
    },
    Calc {
        name: "Monthly saving for a goal", symbol: "goal", keywords: "savings goal monthly deposit needed target future value business", group: BIZ,
        inputs: &[n("Goal", "20000"), n("Yearly return (%)", "4"), n("Years", "5")], result: "Save per month", suffix: "", out: Kind::Currency,
        formula: "goal × r ÷ ((1 + r)^n − 1),  r = rate ÷ 1200, n = years × 12",
        compute: |v| {
            let months = &v[2] * int(12);
            positive(&months, "Years must be above zero")?;
            let r = &v[1] / int(1200);
            if r.is_zero() {
                return Ok(&v[0] / months);
            }
            let g = power(&(R::one() + &r), &months)? - R::one();
            nonzero(&g, "Rate is too small")?;
            Ok(&v[0] * r / g)
        },
    },
    Calc {
        name: "APY from APR", symbol: "APY", keywords: "apy apr effective annual rate interest compounding ear business bank", group: BIZ,
        inputs: &[n("APR (%)", "5"), n("Compounding per year", "12")], result: "APY", suffix: "%", out: Kind::Linear,
        formula: "((1 + APR ÷ n)^n − 1) × 100",
        compute: |v| {
            positive(&v[1], "Compounding must be at least once a year")?;
            let g = power(&(R::one() + pct(&v[0]) / &v[1]), &v[1])?;
            Ok((g - R::one()) * int(100))
        },
    },
    Calc {
        name: "Straight-line depreciation", symbol: "depr.", keywords: "depreciation straight line asset yearly salvage value accounting business", group: BIZ,
        inputs: &[n("Cost", "10000"), n("Salvage value", "1000"), n("Useful life (years)", "5")], result: "Per year", suffix: "", out: Kind::Currency,
        formula: "(cost − salvage) ÷ years",
        compute: |v| positive(&v[2], "Life must be above zero").map(|_| (&v[0] - &v[1]) / &v[2]),
    },
    Calc {
        name: "Customer lifetime value", symbol: "LTV", keywords: "ltv clv customer lifetime value churn arpu saas subscription business", group: BIZ,
        inputs: &[n("Revenue per customer per month", "30"), n("Gross margin (%)", "80"), n("Monthly churn (%)", "5")], result: "Lifetime value", suffix: "", out: Kind::Currency,
        formula: "revenue × margin ÷ churn",
        compute: |v| {
            positive(&v[2], "Churn must be above zero")?;
            Ok(&v[0] * pct(&v[1]) / pct(&v[2]))
        },
    },
    Calc {
        name: "Payback period", symbol: "payback", keywords: "payback period investment return months break even business", group: BIZ,
        inputs: &[n("Investment", "12000"), n("Profit per month", "1000")], result: "Payback", suffix: " months", out: Kind::Linear,
        formula: "investment ÷ monthly profit",
        compute: |v| positive(&v[1], "Profit must be above zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Freelance hourly rate", symbol: "rate/h", keywords: "freelance hourly rate contractor consultant income billable hours business", group: BIZ,
        inputs: &[n("Yearly income wanted", "60000"), n("Billable hours per week", "25"), n("Working weeks per year", "46")], result: "Hourly rate", suffix: "", out: Kind::Currency,
        formula: "income ÷ (hours × weeks)",
        compute: |v| {
            let h = &v[1] * &v[2];
            positive(&h, "Hours and weeks must be above zero")?;
            Ok(&v[0] / h)
        },
    },
    Calc {
        name: "Weekly pay with overtime", symbol: "overtime", keywords: "overtime pay wage hourly time and a half salary business", group: BIZ,
        inputs: &[n("Hourly rate", "20"), n("Hours worked", "45"), n("Overtime after (hours)", "40"), n("Overtime multiplier", "1.5")], result: "Pay", suffix: "", out: Kind::Currency,
        formula: "rate × regular hours + rate × multiplier × overtime hours",
        compute: |v| {
            let regular = if v[1] < v[2] { v[1].clone() } else { v[2].clone() };
            let extra = &v[1] - &regular;
            Ok(&v[0] * regular + &v[0] * &v[3] * extra)
        },
    },
    Calc {
        name: "Cloud cost per month", symbol: "$/h→mo", keywords: "cloud server vps instance hourly price monthly cost aws gcp azure hosting", group: BIZ,
        inputs: &[n("Price per hour", "0.05"), n("Instances", "1")], result: "Per month (730 h)", suffix: "", out: Kind::Currency,
        formula: "price × 730 hours × instances",
        compute: |v| Ok(&v[0] * int(730) * &v[1]),
    },
    // ── Programming & servers ──
    Calc {
        name: "Allowed downtime (SLA)", symbol: "SLA", keywords: "sla uptime availability nines downtime 99.9 99.99 reliability sre", group: DEV,
        inputs: &[n("Availability (%)", "99.9"), n("Period (days)", "365")], result: "Allowed downtime", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "period × (1 − availability ÷ 100)",
        compute: |v| {
            share(&v[0], "Availability is between 0 and 100%")?;
            Ok(&v[1] * int(86400) * (R::one() - pct(&v[0])))
        },
    },
    Calc {
        name: "Availability from downtime", symbol: "uptime %", keywords: "availability uptime downtime sla nines reliability sre", group: DEV,
        inputs: &[n("Downtime (minutes)", "43.2"), n("Period (days)", "30")], result: "Availability", suffix: "%", out: Kind::Linear,
        formula: "(1 − downtime ÷ period) × 100",
        compute: |v| {
            positive(&v[1], "Period must be above zero")?;
            Ok((R::one() - &v[0] / (&v[1] * int(1440))) * int(100))
        },
    },
    Calc {
        name: "Concurrent requests (Little's law)", symbol: "L = λW", keywords: "little's law concurrency in flight requests latency throughput queue capacity planning", group: DEV,
        inputs: &[n("Requests per second", "500"), n("Average latency (ms)", "200")], result: "Requests in flight", suffix: "", out: Kind::Linear,
        formula: "rate × latency",
        compute: |v| Ok(&v[0] * &v[1] / int(1000)),
    },
    Calc {
        name: "Max throughput of workers", symbol: "workers", keywords: "throughput workers threads latency requests per second capacity planning", group: DEV,
        inputs: &[n("Workers / threads", "16"), n("Time per request (ms)", "50")], result: "Max requests", suffix: " /s", out: Kind::Linear,
        formula: "workers × 1000 ÷ latency",
        compute: |v| positive(&v[1], "Latency must be above zero").map(|_| &v[0] * int(1000) / &v[1]),
    },
    Calc {
        name: "Bandwidth-delay product", symbol: "BDP", keywords: "bandwidth delay product tcp window buffer size rtt latency network", group: DEV,
        inputs: &[n("Bandwidth (Mbit/s)", "1000"), n("Round trip (ms)", "50")], result: "In flight", suffix: " KB", out: Kind::Linear,
        formula: "Mbit/s × RTT ms ÷ 8",
        compute: |v| Ok(&v[0] * &v[1] / int(8)),
    },
    Calc {
        name: "Minimum ping over fiber", symbol: "ping", keywords: "ping latency round trip distance fiber optic speed of light network km", group: DEV,
        inputs: &[n("Distance (km)", "1000")], result: "Best round trip", suffix: " ms", out: Kind::Linear,
        formula: "2 × distance ÷ (c ÷ 1.468)  (light in glass, straight line)",
        compute: |v| Ok(int(2) * &v[0] * int(1468) / int(1000) / super::expr("c") * int(1_000_000)),
    },
    Calc {
        name: "Usable IPv4 hosts in a subnet", symbol: "/24 hosts", keywords: "subnet cidr prefix hosts ipv4 netmask network addresses", group: DEV,
        inputs: &[n("Prefix length (/n)", "24")], result: "Usable hosts", suffix: "", out: Kind::Linear,
        formula: "2^(32 − n) − 2  (/31 has 2, /32 has 1)",
        compute: |v| {
            let p = whole(&v[0], 32, "Prefix is 0 to 32")?;
            let all = 1i64 << (32 - p);
            Ok(int(if p >= 31 { all } else { all - 2 }))
        },
    },
    Calc {
        name: "Addresses in an IPv6 prefix", symbol: "/64 size", keywords: "ipv6 prefix addresses subnet cidr network", group: DEV,
        inputs: &[n("Prefix length (/n)", "64")], result: "Addresses", suffix: "", out: Kind::Linear,
        formula: "2^(128 − n)",
        compute: |v| {
            let p = whole(&v[0], 128, "Prefix is 0 to 128")?;
            Ok(R::from_integer(BigInt::one() << (128 - p)))
        },
    },
    Calc {
        name: "Hash collision chance (birthday)", symbol: "collide", keywords: "birthday paradox hash collision probability uuid id bits random", group: DEV,
        inputs: &[n("Number of items", "1000000"), n("Bits of randomness", "64")], result: "Chance of any collision", suffix: "%", out: Kind::Linear,
        formula: "1 − e^(−n² ÷ 2^(bits+1))",
        compute: |v| {
            let (n, bits) = (fl(&v[0])?, fl(&v[1])?);
            if n < 0.0 || bits <= 0.0 {
                return Err("Use positive values");
            }
            from_fl(-(-(n * n) / 2f64.powf(bits + 1.0)).exp_m1() * 100.0)
        },
    },
    Calc {
        name: "Password entropy", symbol: "bits", keywords: "password entropy strength bits random characters length security", group: DEV,
        inputs: &[n("Characters to choose from", "62"), n("Length", "16")], result: "Entropy", suffix: " bits", out: Kind::Linear,
        formula: "length × log₂(characters)  (for truly random passwords)",
        compute: |v| {
            positive(&v[0], "Character count must be above zero")?;
            from_fl(fl(&v[1])? * fl(&v[0])?.log2())
        },
    },
    Calc {
        name: "Bits needed for N values", symbol: "⌈log₂N⌉", keywords: "bits needed values range integer field width log2 encoding", group: DEV,
        inputs: &[n("Number of different values", "1000")], result: "Bits", suffix: "", out: Kind::Linear,
        formula: "⌈log₂ N⌉",
        compute: |v| {
            let n = integer(&v[0], "Use a whole number")?;
            if n < BigInt::one() {
                return Err("Use at least 1 value");
            }
            Ok(int((n - BigInt::one()).bits() as i64))
        },
    },
    Calc {
        name: "Base64 encoded size", symbol: "b64 size", keywords: "base64 encoded size bytes overhead data uri email", group: DEV,
        inputs: &[n("Input size (bytes)", "1000")], result: "Encoded size", suffix: " bytes", out: Kind::Linear,
        formula: "4 × ⌈bytes ÷ 3⌉",
        compute: |v| {
            let n = integer(&v[0], "Use a whole number of bytes")?;
            Ok(R::from_integer((n + BigInt::from(2)) / BigInt::from(3) * BigInt::from(4)))
        },
    },
    Calc {
        name: "RAID usable capacity", symbol: "RAID", keywords: "raid 0 1 5 6 10 usable capacity disks storage nas zfs raidz linux", group: DEV,
        inputs: &[n("Number of disks", "4"), n("Disk size (TB)", "8"), n("RAID level (0, 1, 5, 6, 10)", "5")], result: "Usable", suffix: " TB", out: Kind::Linear,
        formula: "0: n·s · 1: s · 5: (n−1)·s · 6: (n−2)·s · 10: n·s ÷ 2",
        compute: |v| {
            let disks = whole(&v[0], 10_000, "Use a whole number of disks")?;
            let level = whole(&v[2], 10, "RAID level is 0, 1, 5, 6 or 10")?;
            let (min, data) = match level {
                0 => (1, disks),
                1 => (2, 1),
                5 => (3, disks - 1),
                6 => (4, disks - 2),
                10 => (4, disks / 2),
                _ => return Err("RAID level is 0, 1, 5, 6 or 10"),
            };
            if disks < min {
                return Err("Not enough disks for that level");
            }
            Ok(int(data) * &v[1])
        },
    },
    Calc {
        name: "Disk throughput from IOPS", symbol: "IOPS", keywords: "iops throughput disk ssd block size mb/s storage fio", group: DEV,
        inputs: &[n("IOPS", "10000"), n("Block size (KiB)", "4")], result: "Throughput", suffix: " MB/s", out: Kind::Linear,
        formula: "IOPS × KiB × 1024 ÷ 1,000,000",
        compute: |v| Ok(&v[0] * &v[1] * int(1024) / int(1_000_000)),
    },
    Calc {
        name: "Signal from exit code", symbol: "128+n", keywords: "exit code status signal 137 sigkill 139 sigsegv 143 sigterm oom killed linux shell", group: DEV,
        inputs: &[n("Exit code", "137")], result: "Signal number (9 = KILL, 15 = TERM, 11 = SEGV)", suffix: "", out: Kind::Linear,
        formula: "exit code − 128 (codes above 128 mean the process was killed by a signal)",
        compute: |v| {
            let code = whole(&v[0], 255, "Exit codes are 0 to 255")?;
            if code <= 128 {
                return Err("Codes up to 128 are normal exits, not signals");
            }
            Ok(int(code - 128))
        },
    },
    // ── Gaming & displays ──
    Calc {
        name: "Mouse distance per 360°", symbol: "cm/360", keywords: "cm/360 mouse sensitivity dpi yaw 360 turn fps gaming aim", group: GAME,
        inputs: &[n("Mouse DPI", "800"), n("In-game sensitivity", "1.5"), n("Degrees per count (yaw, often 0.022)", "0.022")], result: "Distance for a full turn", suffix: " cm", out: Kind::Linear,
        formula: "360 ÷ (DPI × sensitivity × yaw) × 2.54",
        compute: |v| {
            let d = &v[0] * &v[1] * &v[2];
            positive(&d, "All values must be above zero")?;
            Ok(int(360) / d * int(254) / int(100))
        },
    },
    Calc {
        name: "Sensitivity between games", symbol: "sens", keywords: "sensitivity converter games yaw fps gaming mouse aim", group: GAME,
        inputs: &[n("Sensitivity in game A", "1.5"), n("Yaw of game A (°/count)", "0.022"), n("Yaw of game B (°/count)", "0.07")], result: "Sensitivity in game B", suffix: "", out: Kind::Linear,
        formula: "sensitivity × yaw A ÷ yaw B  (same cm/360)",
        compute: |v| positive(&v[2], "Yaw must be above zero").map(|_| &v[0] * &v[1] / &v[2]),
    },
    Calc {
        name: "Sensitivity after DPI change", symbol: "sens DPI", keywords: "sensitivity dpi change new mouse keep same feel gaming", group: GAME,
        inputs: &[n("Current sensitivity", "2"), n("Old DPI", "400"), n("New DPI", "1600")], result: "New sensitivity", suffix: "", out: Kind::Linear,
        formula: "sensitivity × old DPI ÷ new DPI",
        compute: |v| positive(&v[2], "DPI must be above zero").map(|_| &v[0] * &v[1] / &v[2]),
    },
    Calc {
        name: "eDPI", symbol: "eDPI", keywords: "edpi effective dpi sensitivity mouse gaming", group: GAME,
        inputs: &[n("Mouse DPI", "800"), n("In-game sensitivity", "0.4")], result: "eDPI", suffix: "", out: Kind::Linear,
        formula: "DPI × sensitivity",
        compute: |v| Ok(&v[0] * &v[1]),
    },
    Calc {
        name: "Horizontal FOV from vertical", symbol: "hFOV", keywords: "fov field of view horizontal vertical aspect ratio game ultrawide", group: GAME,
        inputs: &[n("Vertical FOV (°)", "59"), n("Aspect ratio (w/h)", "16/9")], result: "Horizontal FOV", suffix: "°", out: Kind::Linear,
        formula: "2 × atan(tan(vFOV ÷ 2) × aspect)",
        compute: |v| from_fl(2.0 * ((deg(&v[0])? / 2.0).tan() * fl(&v[1])?).atan().to_degrees()),
    },
    Calc {
        name: "Vertical FOV from horizontal", symbol: "vFOV", keywords: "fov field of view horizontal vertical aspect ratio game", group: GAME,
        inputs: &[n("Horizontal FOV (°)", "90"), n("Aspect ratio (w/h)", "16/9")], result: "Vertical FOV", suffix: "°", out: Kind::Linear,
        formula: "2 × atan(tan(hFOV ÷ 2) ÷ aspect)",
        compute: |v| {
            positive(&v[1], "Aspect ratio must be above zero")?;
            from_fl(2.0 * ((deg(&v[0])? / 2.0).tan() / fl(&v[1])?).atan().to_degrees())
        },
    },
    Calc {
        name: "FOV for another aspect ratio", symbol: "FOV a→b", keywords: "fov aspect ratio 4:3 16:9 21:9 ultrawide hor+ field of view game", group: GAME,
        inputs: &[n("Horizontal FOV (°)", "90"), n("At aspect ratio", "4/3"), n("New aspect ratio", "16/9")], result: "Same view at new ratio", suffix: "°", out: Kind::Linear,
        formula: "2 × atan(tan(FOV ÷ 2) × new ÷ old)",
        compute: |v| {
            positive(&v[1], "Aspect ratio must be above zero")?;
            from_fl(2.0 * ((deg(&v[0])? / 2.0).tan() * fl(&(&v[2] / &v[1]))?).atan().to_degrees())
        },
    },
    Calc {
        name: "Screen pixel density (PPI)", symbol: "PPI", keywords: "ppi pixel density dpi screen monitor phone resolution diagonal inch display", group: GAME,
        inputs: &[n("Width (px)", "2560"), n("Height (px)", "1440"), n("Diagonal (inches)", "27")], result: "Pixel density", suffix: " PPI", out: Kind::Linear,
        formula: "√(width² + height²) ÷ diagonal",
        compute: |v| {
            positive(&v[2], "Diagonal must be above zero")?;
            from_fl(fl(&v[0])?.hypot(fl(&v[1])?) / fl(&v[2])?)
        },
    },
    Calc {
        name: "Screen width from diagonal", symbol: "W screen", keywords: "screen monitor tv width height size diagonal aspect ratio inch desk display", group: GAME,
        inputs: &[n("Diagonal (inches)", "27"), n("Aspect ratio (w/h)", "16/9")], result: "Width", suffix: " cm", out: Kind::Linear,
        formula: "diagonal × aspect ÷ √(aspect² + 1) × 2.54",
        compute: |v| {
            let a = fl(&v[1])?;
            from_fl(fl(&v[0])? * a / (a * a + 1.0).sqrt() * 2.54)
        },
    },
    Calc {
        name: "Video signal bandwidth", symbol: "Gbit/s", keywords: "hdmi displayport cable bandwidth resolution refresh rate bit depth monitor video signal 4k 144hz", group: GAME,
        inputs: &[n("Width (px)", "3840"), n("Height (px)", "2160"), n("Refresh rate (Hz)", "144"), n("Bits per pixel (24 = 8-bit, 30 = 10-bit)", "30")], result: "Raw pixel data", suffix: " Gbit/s", out: Kind::Linear,
        formula: "width × height × Hz × bits  (cables also need ~5–20% for blanking)",
        compute: |v| Ok(&v[0] * &v[1] * &v[2] * &v[3] / int(1_000_000_000)),
    },
    Calc {
        name: "Texture memory", symbol: "VRAM", keywords: "texture memory vram size gpu mipmaps rgba game graphics", group: GAME,
        inputs: &[n("Width (px)", "4096"), n("Height (px)", "4096"), n("Bytes per pixel (RGBA8 = 4)", "4"), n("Mipmaps (1 = yes, 0 = no)", "1")], result: "Memory", suffix: " MiB", out: Kind::Linear,
        formula: "width × height × bytes (× 4⁄3 with mipmaps)",
        compute: |v| {
            let base = &v[0] * &v[1] * &v[2] / int(1 << 20);
            Ok(if v[3].is_zero() { base } else { base * int(4) / int(3) })
        },
    },
    Calc {
        name: "Render resolution from upscaling", symbol: "render px", keywords: "dlss fsr xess upscaling render resolution scale internal quality balanced performance game", group: GAME,
        inputs: &[n("Output width or height (px)", "2160"), n("Render scale (%: quality 66.7, balanced 58, performance 50)", "66.7")], result: "Rendered at", suffix: " px", out: Kind::Linear,
        formula: "size × scale (use it once for the width and once for the height)",
        compute: |v| Ok((&v[0] * pct(&v[1])).round()),
    },
    Calc {
        name: "Chance of a drop in N tries", symbol: "drop %", keywords: "drop chance probability loot gacha rng tries at least once game", group: GAME,
        inputs: &[n("Chance per try (%)", "1"), n("Tries", "100")], result: "Chance of at least one", suffix: "%", out: Kind::Linear,
        formula: "1 − (1 − p)^tries",
        compute: |v| {
            let p = share(&v[0], "Chance is between 0 and 100%")?;
            from_fl((1.0 - (1.0 - p).powf(fl(&v[1])?)) * 100.0)
        },
    },
    Calc {
        name: "Tries needed for a drop", symbol: "tries", keywords: "drop chance probability loot gacha rng tries needed pity game", group: GAME,
        inputs: &[n("Chance per try (%)", "1"), n("Wanted certainty (%)", "90")], result: "Tries", suffix: "", out: Kind::Linear,
        formula: "⌈ln(1 − wanted) ÷ ln(1 − p)⌉",
        compute: |v| {
            let p = share(&v[0], "Chance is between 0 and 100%")?;
            let want = share(&v[1], "Certainty is between 0 and 100%")?;
            if p <= 0.0 || want >= 1.0 {
                return Err("Chance must be above 0% and certainty below 100%");
            }
            if p >= 1.0 {
                return Ok(R::one());
            }
            from_fl(((1.0 - want).ln() / (1.0 - p).ln()).ceil())
        },
    },
    Calc {
        name: "Elo win chance", symbol: "Elo", keywords: "elo rating win probability expected score chess mmr ranked game", group: GAME,
        inputs: &[n("Your rating", "1600"), n("Opponent rating", "1500")], result: "Expected score", suffix: "%", out: Kind::Linear,
        formula: "1 ÷ (1 + 10^((opponent − you) ÷ 400))",
        compute: |v| from_fl(100.0 / (1.0 + 10f64.powf(fl(&(&v[1] - &v[0]))? / 400.0))),
    },
    Calc {
        name: "Damage per second", symbol: "DPS", keywords: "dps damage per second fire rate rpm weapon game", group: GAME,
        inputs: &[n("Damage per hit", "30"), n("Fire rate (per minute)", "600")], result: "DPS", suffix: "", out: Kind::Linear,
        formula: "damage × rate ÷ 60",
        compute: |v| Ok(&v[0] * &v[1] / int(60)),
    },
    Calc {
        name: "Nether coordinates", symbol: "÷8", keywords: "minecraft nether overworld coordinates portal travel block game", group: GAME,
        inputs: &[n("Overworld X or Z", "800")], result: "Nether X or Z", suffix: "", out: Kind::Linear,
        formula: "overworld ÷ 8 (multiply by 8 to go back)",
        compute: |v| Ok(&v[0] / int(8)),
    },
    // ── Home & DIY ──
    Calc {
        name: "Paint needed", symbol: "paint", keywords: "paint needed liters wall area coats coverage decorating home diy", group: HOME,
        inputs: &[n("Wall area (m²)", "40"), n("Coats", "2"), n("Coverage (m² per liter)", "10")], result: "Paint", suffix: " L", out: Kind::Linear,
        formula: "area × coats ÷ coverage",
        compute: |v| positive(&v[2], "Coverage must be above zero").map(|_| &v[0] * &v[1] / &v[2]),
    },
    Calc {
        name: "Tiles needed", symbol: "tiles", keywords: "tiles flooring laminate needed count area waste bathroom home diy", group: HOME,
        inputs: &[n("Area (m²)", "10"), n("Tile width (cm)", "30"), n("Tile length (cm)", "60"), n("Extra for cuts (%)", "10")], result: "Tiles", suffix: "", out: Kind::Linear,
        formula: "⌈area × (1 + extra) ÷ tile area⌉",
        compute: |v| {
            let tile = &v[1] * &v[2] / int(10_000);
            positive(&tile, "Tile size must be above zero")?;
            Ok((&v[0] * (R::one() + pct(&v[3])) / tile).ceil())
        },
    },
    Calc {
        name: "Concrete volume", symbol: "m³", keywords: "concrete slab volume cubic meters length width depth footing home diy", group: HOME,
        inputs: &[n("Length (m)", "3"), n("Width (m)", "3"), n("Depth (cm)", "10")], result: "Volume", suffix: " m³", out: Kind::Linear,
        formula: "length × width × depth",
        compute: |v| Ok(&v[0] * &v[1] * &v[2] / int(100)),
    },
    Calc {
        name: "Scale a recipe", symbol: "servings", keywords: "recipe scale servings portions ingredients cooking baking", group: HOME,
        inputs: &[n("Amount in recipe", "250"), n("Recipe serves", "4"), n("You want to serve", "6")], result: "Amount", suffix: "", out: Kind::Linear,
        formula: "amount × wanted ÷ original",
        compute: |v| nonzero(&v[1], "Servings can't be zero").map(|_| &v[0] * &v[2] / &v[1]),
    },
    Calc {
        name: "Sale price", symbol: "sale", keywords: "sale discount price off coupon percent extra stacked shopping", group: DAY,
        inputs: &[n("Price", "80"), n("Discount (%)", "25"), n("Extra discount (%)", "0")], result: "You pay", suffix: "", out: Kind::Currency,
        formula: "price × (1 − d₁) × (1 − d₂)",
        compute: |v| Ok(&v[0] * (R::one() - pct(&v[1])) * (R::one() - pct(&v[2]))),
    },
    Calc {
        name: "Travel time", symbol: "km÷km/h", keywords: "travel time trip duration distance speed drive arrive", group: DAY,
        inputs: &[n("Distance (km)", "300"), n("Average speed (km/h)", "90")], result: "Time", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "distance ÷ speed",
        compute: |v| positive(&v[1], "Speed must be above zero").map(|_| &v[0] / &v[1] * int(3600)),
    },
    Calc {
        name: "Average speed", symbol: "avg km/h", keywords: "average speed distance time trip km/h", group: DAY,
        inputs: &[n("Distance (km)", "42.195"), n("Time (minutes)", "180")], result: "Speed", suffix: " km/h", out: Kind::Linear,
        formula: "distance ÷ hours",
        compute: |v| positive(&v[1], "Time must be above zero").map(|_| &v[0] * int(60) / &v[1]),
    },
    Calc {
        name: "Running pace", symbol: "pace", keywords: "running pace min/km per kilometer marathon 5k 10k jogging time distance", group: DAY,
        inputs: &[n("Distance (km)", "10"), n("Time (minutes)", "55")], result: "Pace per km", suffix: "", out: Kind::Duration(DurFmt::Clock),
        formula: "time ÷ distance",
        compute: |v| positive(&v[0], "Distance must be above zero").map(|_| &v[1] * int(60) / &v[0]),
    },
    // ── More health ──
    Calc {
        name: "Waist-to-height ratio", symbol: "WHtR", keywords: "waist to height ratio whtr health weight belly", group: HEALTH,
        inputs: &[n("Waist (cm)", "80"), n("Height (cm)", "175")], result: "Ratio", suffix: "", out: Kind::Linear,
        formula: "waist ÷ height  (under 0.5 is the usual healthy range)",
        compute: |v| positive(&v[1], "Height must be above zero").map(|_| &v[0] / &v[1]),
    },
    Calc {
        name: "Heart rate training zone", symbol: "HR zone", keywords: "heart rate zone karvonen training cardio target bpm fitness", group: HEALTH,
        inputs: &[n("Age", "30"), n("Resting heart rate", "60"), n("Intensity (%)", "70")], result: "Target", suffix: " bpm", out: Kind::Linear,
        formula: "Karvonen: (220 − age − rest) × intensity + rest",
        compute: |v| Ok(((int(220) - &v[0] - &v[1]) * pct(&v[2]) + &v[1]).round()),
    },
    Calc {
        name: "Calories burned (MET)", symbol: "MET kcal", keywords: "calories burned exercise met activity workout walking running cycling fitness", group: HEALTH,
        inputs: &[n("MET (walk 3.5, run 9.8, cycle 7.5)", "3.5"), n("Weight (kg)", "70"), n("Minutes", "30")], result: "Calories", suffix: " kcal", out: Kind::Linear,
        formula: "MET × kg × hours",
        compute: |v| Ok(&v[0] * &v[1] * &v[2] / int(60)),
    },
    Calc {
        name: "Body surface area", symbol: "BSA", keywords: "body surface area bsa mosteller medicine dose health", group: HEALTH,
        inputs: &[n("Height (cm)", "175"), n("Weight (kg)", "70")], result: "Surface area", suffix: " m²", out: Kind::Linear,
        formula: "Mosteller: √(cm × kg ÷ 3600)",
        compute: |v| {
            let x = fl(&(&v[0] * &v[1]))?;
            if x < 0.0 {
                return Err("Use positive values");
            }
            from_fl((x / 3600.0).sqrt())
        },
    },
];

pub fn register(b: &mut Builder) {
    for (g, &(id, name)) in GROUPS.iter().enumerate() {
        b.category(id, name, false);
        for (i, c) in CALCS.iter().enumerate().filter(|(_, c)| c.group == g) {
            b.add(c.name, c.symbol, Kind::Calc(i), c.keywords, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::num::eval_input;

    fn run(name: &str, args: &[&str]) -> String {
        run_sig(name, args, 10)
    }

    fn run_sig(name: &str, args: &[&str], sig: usize) -> String {
        let c = CALCS.iter().find(|c| c.name == name).unwrap();
        assert_eq!(c.inputs.len(), args.len(), "{name}");
        let v: Vec<R> = args.iter().map(|a| eval_input(a).unwrap()).collect();
        let r = (c.compute)(&v).unwrap();
        crate::convert::format_value(c.out, &r, sig, crate::num::Notation::Auto).0
    }

    #[test]
    fn percentages() {
        assert_eq!(run("X% of Y", &["15", "80"]), "12");
        assert_eq!(run("X is what % of Y", &["12", "80"]), "15");
        assert_eq!(run("Percentage change", &["80", "100"]), "25");
        assert_eq!(run("Percentage change", &["-50", "-25"]), "50");
        assert_eq!(run("Increase by %", &["100", "20"]), "120");
        assert_eq!(run("Decrease by %", &["100", "20"]), "80");
        assert_eq!(run("Value before % increase", &["120", "20"]), "100");
        assert_eq!(run("X is P% of what", &["30", "15"]), "200");
        assert_eq!(run("Profit margin", &["60", "100"]), "40");
        assert_eq!(run("Markup", &["60", "100"]), "66.66666667");
        assert_eq!(run("Price for target margin", &["60", "40"]), "100");
    }

    #[test]
    fn math_and_science() {
        assert_eq!(run("Greatest common divisor", &["84", "36"]), "12");
        assert_eq!(run("Least common multiple", &["4", "6"]), "12");
        assert_eq!(run_sig("Factorial", &["20"], 30), "2432902008176640000");
        assert_eq!(run("Combinations (n choose k)", &["49", "6"]), "13983816");
        assert_eq!(run("Permutations", &["10", "3"]), "720");
        assert_eq!(run("Power", &["2", "10"]), "1024");
        assert_eq!(run("Root", &["-27", "3"]), "-3");
        assert_eq!(run("Logarithm", &["1000", "10"]), "3");
        assert_eq!(run("Modulo (remainder)", &["-7", "3"]), "2");
        assert_eq!(run("Hypotenuse", &["3", "4"]), "5");
        assert_eq!(run("Rule of three", &["2", "300", "5"]), "750");
        assert_eq!(run("Voltage divider", &["12", "10000", "10000"]), "6");
        assert_eq!(run("LED series resistor", &["5", "2", "20"]), "150");
        assert_eq!(run("Resistors in parallel", &["100", "100"]), "50");
        assert_eq!(run("Dilution (C₁V₁ = C₂V₂)", &["2", "50", "0.5"]), "200");
        assert_eq!(run("Ideal gas pressure", &["1", "273.15", "22.414"]), "101.3248623");
        assert_eq!(run("Battery runtime", &["3000", "250"]), "12:00:00");
    }

    #[test]
    fn new_calculators() {
        assert_eq!(run("Resistors in series", &["1000", "2200", "0", "0"]), "3200");
        assert_eq!(run("Capacitors in series", &["10", "10"]), "5");
        assert_eq!(run("Nearest standard resistor value", &["5000", "12"]), "4700");
        assert_eq!(run("LM317 output voltage", &["240", "720"]), "5");
        assert_eq!(run("555 timer duty cycle (astable)", &["1000", "1000"]), "66.66666667");
        assert_eq!(run_sig("PCB trace width", &["1", "10", "1"], 3), "0.3");
        assert_eq!(run("Kinetic energy", &["2", "3"]), "9");
        assert_eq!(run_sig("Escape velocity", &["5.972e24", "6.371e6"], 4), "11190");
        assert_eq!(run("Remove VAT / sales tax", &["120", "20"]), "100.00");
        assert_eq!(run("Break-even point", &["10000", "50", "30"]), "500");
        assert_eq!(run_sig("Compound annual growth rate (CAGR)", &["1000", "2000", "1"], 4), "100");
        assert_eq!(run("Savings with monthly deposits", &["100", "0", "1", "50"]), "1250.00");
        assert_eq!(run("Allowed downtime (SLA)", &["99.9", "365"]), "8:45:36");
        assert_eq!(run("Usable IPv4 hosts in a subnet", &["24"]), "254");
        assert_eq!(run("Usable IPv4 hosts in a subnet", &["32"]), "1");
        assert_eq!(run("Bits needed for N values", &["256"]), "8");
        assert_eq!(run("Bits needed for N values", &["257"]), "9");
        assert_eq!(run("Base64 encoded size", &["10"]), "16");
        assert_eq!(run("RAID usable capacity", &["4", "8", "6"]), "16");
        assert_eq!(run("Signal from exit code", &["137"]), "9");
        assert_eq!(run_sig("Mouse distance per 360°", &["800", "1.5", "0.022"], 4), "34.64");
        assert_eq!(run("Sensitivity between games", &["1.5", "0.022", "0.066"]), "0.5");
        assert_eq!(run_sig("FOV for another aspect ratio", &["90", "4/3", "16/9"], 5), "106.26");
        assert_eq!(run_sig("Chance of a drop in N tries", &["1", "100"], 4), "63.4");
        assert_eq!(run("Tries needed for a drop", &["1", "90"]), "230");
        assert_eq!(run("Elo win chance", &["1500", "1500"]), "50");
        assert_eq!(run("Tiles needed", &["10", "30", "60", "10"]), "62");
        assert_eq!(run("Running pace", &["10", "55"]), "0:05:30");
        assert_eq!(run("Fibonacci number", &["10"]), "55");
        assert_eq!(run("Weekly pay with overtime", &["20", "45", "40", "1.5"]), "950.00");
    }

    #[test]
    fn everyday() {
        assert_eq!(run("Tip and split", &["100", "20", "4"]), "30.00");
        assert_eq!(run("Loan payment", &["100000", "6", "30"]), "599.55");
        assert_eq!(run("Loan payment", &["1200", "0", "1"]), "100.00");
        assert_eq!(run("Compound interest", &["1000", "10", "2", "1"]), "1210.00");
        assert_eq!(run("Download time", &["1", "8"]), "0:16:40");
        assert_eq!(run("Body mass index", &["80", "200"]), "20");
        let iso = |s| DateFmt::Iso.parse(s).unwrap().unwrap();
        let c = CALCS.iter().find(|c| c.name == "Days between dates").unwrap();
        assert_eq!((c.compute)(&[iso("2026-01-01"), iso("2026-12-25")]).unwrap(), int(358));
        assert!(run("Factorial", &["3000"]).len() > 10);
    }
}
