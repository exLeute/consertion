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
];
const PCT: usize = 0;
const MATH: usize = 1;
const ELEC: usize = 2;
const CHEM: usize = 3;
const DAY: usize = 4;
const HEALTH: usize = 5;
const PHOTO: usize = 6;
const MUSIC: usize = 7;

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
