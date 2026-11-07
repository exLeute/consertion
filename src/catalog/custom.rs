//! Your own units, saved in settings. Each is "name = amount unit": "step = 0.75 m" joins
//! Length and converts to every length unit. Leaving "equals" empty starts a new family
//! ("pizza"), which other units can be defined from ("slice = 1/8 pizza").

use super::{Builder, Catalog, Kind};
use crate::num::{self, R};
use num_traits::{One, Signed, Zero};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CustomUnit {
    pub name: String,
    /// Optional short name; the name is used when empty.
    pub symbol: String,
    /// "0.75 m", "1/8 pizza", or "" for a new family.
    pub equals: String,
}

impl CustomUnit {
    pub fn symbol(&self) -> &str {
        if self.symbol.trim().is_empty() { self.name.trim() } else { self.symbol.trim() }
    }

    pub fn id(&self) -> String {
        format!("custom:{}", slug(&self.name))
    }
}

fn slug(s: &str) -> String {
    s.trim().to_lowercase().chars().map(|c| if c.is_alphanumeric() { c } else { '-' }).collect()
}

/// Families of your own units get tags from here, so pizzas never convert to apples.
const FAMILY_TAG: u16 = 1000;

pub fn register(b: &mut Builder) {
    b.category("custom", "✎ My Converters", false);
    b.add("New custom converter…", "＋ new", Kind::NewCustom, "custom own unit create add define my converter", true);
}

/// Split "1/8 pizza" into ("1/8", "pizza"). The unit is the longest tail that names one.
pub fn split(equals: &str) -> (&str, &str) {
    let s = equals.trim();
    let starts = s.char_indices().filter(|&(i, c)| {
        let prev = s[..i].chars().last();
        i == 0 || prev.is_some_and(char::is_whitespace) || (c.is_alphabetic() && prev.is_some_and(|p| p.is_ascii_digit() || p == '.' || p == ')'))
    });
    for (i, _) in starts {
        let (amount, unit) = (s[..i].trim(), s[i..].trim());
        if !unit.is_empty() && !unit.starts_with(|c: char| c.is_ascii_digit() || "+-*/^().,".contains(c)) {
            return (amount, unit);
        }
    }
    (s, "")
}

/// A plain (linear) unit called `text`: by symbol, then name, ignoring case, then without a plural "s".
fn find(c: &Catalog, text: &str, skip: &str) -> Result<usize, String> {
    let t = text.trim();
    let tl = t.to_lowercase();
    let singular = tl.strip_suffix('s').unwrap_or(&tl).to_string();
    let tries: [&dyn Fn(&super::Unit) -> bool; 4] = [
        &|u| u.symbol == t,
        &|u| u.symbol.to_lowercase() == tl,
        &|u| u.name.to_lowercase() == tl,
        &|u| u.name.to_lowercase() == singular || u.symbol.to_lowercase() == singular,
    ];
    // Plain units first, through every spelling ("ml" is millilitre, not the Malayalam language).
    for test in &tries {
        // Ignoring case, "ml" could be mL or ML: take the everyday one.
        let hits = (0..c.units.len()).filter(|&i| c.units[i].id != skip && c.units[i].kind == Kind::Linear && test(&c.units[i]));
        if let Some(i) = hits.min_by_key(|&i| !c.units[i].common) {
            return Ok(i);
        }
    }
    let other = tries.iter().find_map(|test| (0..c.units.len()).find(|&i| c.units[i].id != skip && test(&c.units[i])));
    Err(match other.map(|i| &c.units[i]) {
        Some(u) if u.kind == Kind::Currency => "Currencies change every day, so they can't define a unit".into(),
        Some(u) => format!("{} isn't a plain unit — use one like m, kg, L or h", u.name),
        None => format!("No unit called \"{t}\" — try a symbol like m, kg, L, or one of your own"),
    })
}

/// Units the typed "equals" could mean, for completing it: plain units whose symbol or name
/// starts with what's typed. Empty once it names a unit exactly.
pub fn suggest(c: &Catalog, equals: &str, skip: &str) -> Vec<usize> {
    let unit = split(equals).1.to_lowercase();
    if unit.is_empty() || c.units.iter().any(|u| u.id != skip && u.kind == Kind::Linear && (u.symbol.to_lowercase() == unit || u.name.to_lowercase() == unit)) {
        return Vec::new();
    }
    let mut hits: Vec<usize> = (0..c.units.len())
        .filter(|&i| {
            let u = &c.units[i];
            u.id != skip && u.kind == Kind::Linear && (u.symbol.to_lowercase().starts_with(&unit) || u.name.to_lowercase().starts_with(&unit))
        })
        .collect();
    hits.sort_by_key(|&i| (!c.units[i].common, c.units[i].symbol.len()));
    hits.dedup_by_key(|i| c.units[*i].symbol.clone());
    hits.truncate(5);
    hits
}

/// "0.75 met" + meter → "0.75 m".
pub fn complete(equals: &str, symbol: &str) -> String {
    format!("{} {symbol}", split(equals).0).trim().to_string()
}

/// What "equals" means: (amount, unit index). Err explains what's wrong.
pub fn resolve(c: &Catalog, equals: &str, skip: &str) -> Result<(R, usize), String> {
    let (amount, unit) = split(equals);
    if unit.is_empty() {
        return Err("Add a unit after the number, e.g. 0.75 m".into());
    }
    let u = find(c, unit, skip)?;
    let amount = if amount.is_empty() {
        R::one()
    } else {
        crate::convert::number(amount)?.ok_or("Finish the number, e.g. 0.75")?
    };
    if !amount.is_positive() || !num::in_range(&amount) {
        return Err("The amount must be a number above zero".into());
    }
    Ok((amount, u))
}

/// Add your units to the catalog. Units whose "equals" no longer resolves are skipped and
/// named in `Catalog::broken`.
pub fn add_all(b: &mut Builder, list: &[CustomUnit]) {
    let custom_cat = b.cat.categories.iter().position(|k| k.id == "custom").unwrap_or(0);
    let mut pending: Vec<usize> = (0..list.len()).collect();
    let mut family = FAMILY_TAG;
    // Units may be defined from later ones, so keep going while anything new resolves.
    loop {
        let before = pending.len();
        pending.retain(|&i| {
            let cu = &list[i];
            let (cat, tag, factor) = if cu.equals.trim().is_empty() {
                family += 1;
                (custom_cat, family, R::one())
            } else {
                match resolve(&b.cat, &cu.equals, &cu.id()) {
                    Ok((amount, u)) => {
                        let r = &b.cat.units[u];
                        (r.category, r.tag, amount * &r.factor)
                    }
                    Err(_) => return true,
                }
            };
            b.current = cat;
            b.tag = tag;
            b.unit(cu.name.trim(), cu.symbol(), Kind::Linear, factor, R::zero(), &format!("custom my own {}", cu.equals), true);
            if let Some(u) = b.cat.units.last_mut() {
                u.id = cu.id();
            }
            false
        });
        if pending.is_empty() || pending.len() == before {
            break;
        }
    }
    b.tag = 0;
    b.cat.broken = pending.iter().map(|&i| list[i].name.clone()).collect();
}

/// The list with `unit` added (or replacing `editing`), units defined from a renamed one
/// following it. Err says why it can't be saved.
pub fn apply(list: &[CustomUnit], editing: Option<usize>, unit: &CustomUnit) -> Result<Vec<CustomUnit>, String> {
    let name = unit.name.trim();
    if name.is_empty() {
        return Err("Give it a name, e.g. step".into());
    }
    if name.chars().count() > 40 || unit.symbol.trim().chars().count() > 16 {
        return Err("Keep the name under 40 and the symbol under 16 characters".into());
    }
    let others = list.iter().enumerate().filter(|(i, _)| Some(*i) != editing).map(|(_, u)| u);
    for o in others.clone() {
        if o.id() == unit.id() {
            return Err(format!("You already have a unit called {}", o.name));
        }
        if o.symbol().eq_ignore_ascii_case(unit.symbol()) {
            return Err(format!("{} already uses the symbol {}", o.name, o.symbol()));
        }
    }
    // A symbol another unit already has would make that unit hard to reach ("mg" is milligram).
    let builtin = Catalog::with_custom(&[]);
    if let Some(u) = builtin.units.iter().find(|u| u.symbol == unit.symbol() || (unit.symbol.trim().is_empty() && u.symbol.eq_ignore_ascii_case(unit.name.trim()))) {
        return Err(format!("{} already means {} — choose another {}", unit.symbol(), u.name, if unit.symbol.trim().is_empty() { "name or add a symbol" } else { "symbol" }));
    }
    // Build with the change applied: catches unknown units, loops and broken dependents.
    let mut trial: Vec<CustomUnit> = list.to_vec();
    match editing {
        Some(i) if i < trial.len() => {
            let old = std::mem::replace(&mut trial[i], unit.clone());
            rename_refs(&mut trial, &old, unit);
        }
        _ => trial.push(unit.clone()),
    }
    let c = Catalog::with_custom(&trial);
    if !unit.equals.trim().is_empty() {
        resolve(&c, &unit.equals, &unit.id())?;
    }
    if !c.broken.is_empty() {
        return Err(format!("That would break {} (a loop, or it refers to a unit that's gone)", c.broken.join(", ")));
    }
    Ok(trial)
}

/// The list without unit `i`, unless others are defined from it.
pub fn remove(list: &[CustomUnit], i: usize) -> Result<Vec<CustomUnit>, String> {
    let unit = list.get(i).ok_or("That unit is already gone")?;
    let deps: Vec<&str> = dependents(list, unit).into_iter().map(|d| list[d].name.as_str()).collect();
    if !deps.is_empty() {
        return Err(format!("{} {} defined from it — change or delete {} first", deps.join(", "), if deps.len() == 1 { "is" } else { "are" }, if deps.len() == 1 { "that" } else { "those" }));
    }
    Ok(list.iter().enumerate().filter(|(j, _)| *j != i).map(|(_, u)| u.clone()).collect())
}

/// Units defined from `unit` (by name, symbol or plural name).
pub fn dependents(list: &[CustomUnit], unit: &CustomUnit) -> Vec<usize> {
    let names = [unit.name.trim().to_lowercase(), unit.symbol().to_lowercase(), format!("{}s", unit.name.trim().to_lowercase())];
    (0..list.len()).filter(|&i| list[i] != *unit && names.contains(&split(&list[i].equals).1.to_lowercase())).collect()
}

/// After renaming `old` to `new`, point the units defined from it at the new symbol.
pub fn rename_refs(list: &mut [CustomUnit], old: &CustomUnit, new: &CustomUnit) {
    for i in dependents(list, old) {
        let amount = split(&list[i].equals).0.to_string();
        list[i].equals = format!("{amount} {}", new.symbol()).trim().to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::{convert, Output};

    fn unit(name: &str, symbol: &str, equals: &str) -> CustomUnit {
        CustomUnit { name: name.into(), symbol: symbol.into(), equals: equals.into() }
    }

    fn conv(c: &Catalog, from: &str, to: &str, v: &str) -> String {
        let (f, t) = (&c.units[c.find(from).unwrap()], &c.units[c.find(to).unwrap()]);
        match convert(f, t, v, None).unwrap() {
            Output::Number(n) => num::format(&n, 10, num::Notation::Auto).0,
            o => format!("{o:?}"),
        }
    }

    #[test]
    fn custom_units() {
        assert_eq!(split("0.75 m"), ("0.75", "m"));
        assert_eq!(split("2.5cm"), ("2.5", "cm"));
        assert_eq!(split("1/8 pizza"), ("1/8", "pizza"));
        assert_eq!(split("light year"), ("", "light year"));
        assert_eq!(split("12"), ("12", ""));

        // Defined out of order: slice comes before pizza.
        let list = vec![unit("slice", "", "1/8 pizza"), unit("step", "stp", "0.75 m"), unit("pizza", "", ""), unit("apple", "", "")];
        let c = Catalog::with_custom(&list);
        assert!(c.broken.is_empty());
        assert_eq!(conv(&c, "custom:step", "length:centimeter", "4"), "300");
        assert_eq!(conv(&c, "custom:pizza", "custom:slice", "3"), "24");
        // Different families never pair.
        assert!(!c.pairs(c.find("custom:pizza").unwrap(), c.find("custom:apple").unwrap()));
        assert!(c.pairs(c.find("custom:step").unwrap(), c.find("length:meter").unwrap()));

        assert!(apply(&list, None, &unit("Step", "", "1 m")).is_err(), "duplicate name");
        assert!(apply(&list, None, &unit("hop", "", "2 parsecz")).is_err(), "unknown unit");
        assert!(apply(&list, None, &unit("hop", "", "2 USD")).is_err(), "currency");
        assert!(apply(&list, None, &unit("hop", "", "-1 m")).is_err(), "negative");
        assert!(apply(&list, Some(2), &unit("pizza", "", "1 slice")).is_err(), "loop");
        assert!(apply(&list, None, &unit("hop", "", "3 steps")).is_ok(), "plural of a custom unit");
        let mug = apply(&list, None, &unit("mug", "", "350 ml")).unwrap();
        let c2 = Catalog::with_custom(&mug);
        assert_eq!(conv(&c2, "custom:mug", "volume:liter", "1"), "0.35");
        assert!(apply(&list, None, &unit("mug", "mg", "350 ml")).is_err(), "mg is milligram");

        // Renaming pizza keeps slice working.
        let renamed = apply(&list, Some(2), &unit("pie", "", "")).unwrap();
        assert_eq!(renamed[0].equals, "1/8 pie");
        assert!(Catalog::with_custom(&renamed).broken.is_empty());
        assert_eq!(dependents(&list, &list[2]), vec![0]);
        assert!(remove(&list, 2).is_err(), "slice still needs pizza");
        assert_eq!(remove(&list, 0).unwrap().len(), 3);
    }
}
