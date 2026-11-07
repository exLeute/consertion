//! Chemical formulas: molar mass, mass composition and Hill notation.
//! Atomic weights are IUPAC conventional values; radioactive elements use the mass number of
//! their longest-lived isotope.

use super::{Builder, Kind};
use crate::num::{self, int, R};
use num_traits::Zero;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChemFmt {
    Formula,
    /// g/mol (output only)
    MolarMass,
    /// "C 40.00% · H 6.71% · O 53.29%" (output only)
    Composition,
    /// "C6H12O6" (output only)
    Hill,
    /// Atoms per formula unit (output only)
    Atoms,
}

const ELEMENTS: &[(&str, &str)] = &[
    ("H", "1.008"), ("He", "4.002602"), ("Li", "6.94"), ("Be", "9.0121831"), ("B", "10.81"), ("C", "12.011"),
    ("N", "14.007"), ("O", "15.999"), ("F", "18.998403163"), ("Ne", "20.1797"), ("Na", "22.98976928"),
    ("Mg", "24.305"), ("Al", "26.9815385"), ("Si", "28.085"), ("P", "30.973761998"), ("S", "32.06"),
    ("Cl", "35.45"), ("Ar", "39.948"), ("K", "39.0983"), ("Ca", "40.078"), ("Sc", "44.955908"),
    ("Ti", "47.867"), ("V", "50.9415"), ("Cr", "51.9961"), ("Mn", "54.938044"), ("Fe", "55.845"),
    ("Co", "58.933194"), ("Ni", "58.6934"), ("Cu", "63.546"), ("Zn", "65.38"), ("Ga", "69.723"),
    ("Ge", "72.630"), ("As", "74.921595"), ("Se", "78.971"), ("Br", "79.904"), ("Kr", "83.798"),
    ("Rb", "85.4678"), ("Sr", "87.62"), ("Y", "88.90584"), ("Zr", "91.224"), ("Nb", "92.90637"),
    ("Mo", "95.95"), ("Tc", "98"), ("Ru", "101.07"), ("Rh", "102.90550"), ("Pd", "106.42"),
    ("Ag", "107.8682"), ("Cd", "112.414"), ("In", "114.818"), ("Sn", "118.710"), ("Sb", "121.760"),
    ("Te", "127.60"), ("I", "126.90447"), ("Xe", "131.293"), ("Cs", "132.90545196"), ("Ba", "137.327"),
    ("La", "138.90547"), ("Ce", "140.116"), ("Pr", "140.90766"), ("Nd", "144.242"), ("Pm", "145"),
    ("Sm", "150.36"), ("Eu", "151.964"), ("Gd", "157.25"), ("Tb", "158.92535"), ("Dy", "162.500"),
    ("Ho", "164.93033"), ("Er", "167.259"), ("Tm", "168.93422"), ("Yb", "173.045"), ("Lu", "174.9668"),
    ("Hf", "178.49"), ("Ta", "180.94788"), ("W", "183.84"), ("Re", "186.207"), ("Os", "190.23"),
    ("Ir", "192.217"), ("Pt", "195.084"), ("Au", "196.966569"), ("Hg", "200.592"), ("Tl", "204.38"),
    ("Pb", "207.2"), ("Bi", "208.98040"), ("Po", "209"), ("At", "210"), ("Rn", "222"), ("Fr", "223"),
    ("Ra", "226"), ("Ac", "227"), ("Th", "232.0377"), ("Pa", "231.03588"), ("U", "238.02891"),
    ("Np", "237"), ("Pu", "244"), ("Am", "243"), ("Cm", "247"), ("Bk", "247"), ("Cf", "251"),
    ("Es", "252"), ("Fm", "257"), ("Md", "258"), ("No", "259"), ("Lr", "262"), ("Rf", "267"),
    ("Db", "268"), ("Sg", "269"), ("Bh", "270"), ("Hs", "269"), ("Mt", "278"), ("Ds", "281"),
    ("Rg", "282"), ("Cn", "285"), ("Nh", "286"), ("Fl", "289"), ("Mc", "290"), ("Lv", "293"),
    ("Ts", "294"), ("Og", "294"),
];

pub fn register(b: &mut Builder) {
    b.category("chemistry", "Chemical Formula", false);
    for (f, name, sym, kw) in [
        (ChemFmt::Formula, "Chemical formula", "H₂O", "molecule compound element chemistry"),
        (ChemFmt::MolarMass, "Molar mass (g/mol)", "g/mol", "molecular weight mass"),
        (ChemFmt::Composition, "Mass composition (%)", "wt%", "percent elements"),
        (ChemFmt::Hill, "Hill formula", "CₓHᵧ", "canonical notation"),
        (ChemFmt::Atoms, "Atom count", "atoms", "number of atoms"),
    ] {
        b.add(name, sym, Kind::Chem(f), kw, true);
    }
}

pub fn accepts(s: &str) -> bool {
    s.chars().count() <= 200 && s.chars().all(|c| c.is_ascii_alphanumeric() || "()[]{}·.*• ".contains(c) || ('₀'..='₉').contains(&c))
}

/// Element counts in order of first appearance. `None` while incomplete.
pub fn parse(s: &str) -> Result<Option<Vec<(usize, R)>>, String> {
    // Subscripts → digits; "·", "*", "." separate hydrate parts ("CuSO4·5H2O").
    let s: String = s
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            '₀'..='₉' => char::from(b'0' + (c as u32 - '₀' as u32) as u8),
            '·' | '*' | '•' => '.',
            '[' | '{' => '(',
            ']' | '}' => ')',
            c => c,
        })
        .collect();
    if s.is_empty() {
        return Ok(None);
    }
    let mut total: Vec<(usize, R)> = Vec::new();
    for part in s.split('.') {
        if part.is_empty() {
            return Ok(None);
        }
        let digits = part.chars().take_while(|c| c.is_ascii_digit()).count();
        let coef = if digits > 0 { int(part[..digits].parse::<i64>().map_err(|_| "Coefficient too large")?) } else { int(1) };
        let chars: Vec<char> = part[digits..].chars().collect();
        let mut i = 0;
        let Some(counts) = group(&chars, &mut i)? else { return Ok(None) };
        if i < chars.len() {
            return Err(format!("Unexpected \"{}\"", chars[i]));
        }
        for (e, n) in counts {
            merge(&mut total, e, n * &coef);
        }
    }
    Ok(Some(total))
}

fn merge(into: &mut Vec<(usize, R)>, e: usize, n: R) {
    match into.iter_mut().find(|x| x.0 == e) {
        Some(x) => x.1 += n,
        None => into.push((e, n)),
    }
}

fn count(c: &[char], i: &mut usize) -> Result<R, String> {
    let start = *i;
    while *i < c.len() && c[*i].is_ascii_digit() {
        *i += 1;
    }
    if start == *i {
        return Ok(int(1));
    }
    let n: String = c[start..*i].iter().collect();
    n.parse::<i64>().ok().filter(|&n| n <= 1_000_000).map(int).ok_or("Count too large".into())
}

fn group(c: &[char], i: &mut usize) -> Result<Option<Vec<(usize, R)>>, String> {
    let mut out = Vec::new();
    while *i < c.len() {
        match c[*i] {
            '(' => {
                *i += 1;
                let Some(inner) = group(c, i)? else { return Ok(None) };
                if *i >= c.len() {
                    return Ok(None);
                }
                *i += 1; // ')'
                let n = count(c, i)?;
                for (e, k) in inner {
                    merge(&mut out, e, k * &n);
                }
            }
            ')' => break,
            ch if ch.is_ascii_uppercase() => {
                let mut sym = ch.to_string();
                *i += 1;
                if *i < c.len() && c[*i].is_ascii_lowercase() {
                    sym.push(c[*i]);
                    *i += 1;
                }
                let e = ELEMENTS.iter().position(|x| x.0 == sym).ok_or(format!("Unknown element \"{sym}\""))?;
                let n = count(c, i)?;
                merge(&mut out, e, n);
            }
            ch if ch.is_ascii_lowercase() => return Err(format!("Element symbols start with a capital (\"{}\")", ch.to_ascii_uppercase())),
            ch => return Err(format!("Unexpected \"{ch}\"")),
        }
    }
    Ok(Some(out))
}

pub fn molar_mass(atoms: &[(usize, R)]) -> R {
    atoms.iter().fold(int(0), |m, (e, n)| m + n * num::parse_decimal(ELEMENTS[*e].1).unwrap_or_default())
}

pub fn convert(to: ChemFmt, input: &str, sig: usize) -> Result<Option<crate::convert::Output>, String> {
    use crate::convert::Output;
    let Some(atoms) = parse(input)? else { return Ok(None) };
    if atoms.is_empty() {
        return Ok(None);
    }
    let fmt = |q: &R| num::format(q, sig, num::Notation::Auto).0;
    Ok(Some(match to {
        ChemFmt::MolarMass => Output::Number(molar_mass(&atoms)),
        ChemFmt::Atoms => Output::Number(atoms.iter().fold(int(0), |a, (_, n)| a + n)),
        ChemFmt::Composition => {
            let total = molar_mass(&atoms);
            if total.is_zero() {
                return Ok(None);
            }
            let parts: Vec<String> = atoms
                .iter()
                .map(|(e, n)| {
                    let share = n * num::parse_decimal(ELEMENTS[*e].1).unwrap_or_default() / &total * int(100);
                    format!("{} {}%", ELEMENTS[*e].0, num::format_fixed(&share, 2))
                })
                .collect();
            Output::Text(parts.join(" · "))
        }
        ChemFmt::Hill | ChemFmt::Formula => {
            // Hill order: C, then H, then the rest alphabetically (all alphabetical without carbon).
            let mut sorted = atoms.clone();
            let has_c = sorted.iter().any(|(e, _)| ELEMENTS[*e].0 == "C");
            let rank = |e: usize| match ELEMENTS[e].0 {
                "C" if has_c => (0, ""),
                "H" if has_c => (1, ""),
                s => (2, s),
            };
            sorted.sort_by_key(|(e, _)| rank(*e));
            Output::Text(sorted.iter().map(|(e, n)| if *n == int(1) { ELEMENTS[*e].0.to_string() } else { format!("{}{}", ELEMENTS[*e].0, fmt(n)) }).collect())
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::Output;

    fn mass(s: &str) -> String {
        match convert(ChemFmt::MolarMass, s, 8).unwrap().unwrap() {
            Output::Number(q) => num::format(&q, 8, num::Notation::Auto).0,
            _ => unreachable!(),
        }
    }

    #[test]
    fn formulas() {
        assert_eq!(mass("H2O"), "18.015");
        assert_eq!(mass("C6H12O6"), "180.156");
        assert_eq!(mass("Ca(OH)2"), "74.092");
        assert_eq!(mass("CuSO4·5H2O"), "249.677");
        assert_eq!(mass("H₂SO₄"), "98.072");
        assert_eq!(convert(ChemFmt::Hill, "OH2", 8).unwrap(), Some(Output::Text("H2O".into())));
        assert_eq!(convert(ChemFmt::Hill, "HOCH2CH3", 8).unwrap(), Some(Output::Text("C2H6O".into())));
        assert_eq!(convert(ChemFmt::Composition, "H2O", 8).unwrap(), Some(Output::Text("H 11.19% · O 88.81%".into())));
        assert_eq!(parse("Ca(OH"), Ok(None), "incomplete");
        assert!(parse("Xx2").is_err());
        assert!(parse("h2o").is_err());
    }
}
