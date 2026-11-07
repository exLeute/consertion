//! Map coordinates: a latitude/longitude pair in decimal degrees, degrees-minutes-seconds,
//! degrees and decimal minutes, Open Location Code ("plus code") or geohash.
//! Every degree format reads every other one, so pasting from any map works.

use super::{Builder, Kind};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoordFmt {
    Decimal,
    Dms,
    Ddm,
    PlusCode,
    Geohash,
}

pub fn register(b: &mut Builder) {
    b.category("coords", "Map Coordinates", false);
    b.add("decimal degrees", "DD", Kind::Coord(CoordFmt::Decimal), "latitude longitude lat lon gps google maps location", true);
    b.add("degrees, minutes, seconds", "DMS", Kind::Coord(CoordFmt::Dms), "latitude longitude gps dms sexagesimal", true);
    b.add("degrees and decimal minutes", "DDM", Kind::Coord(CoordFmt::Ddm), "latitude longitude gps marine nautical", true);
    b.add("plus code (Open Location Code)", "plus code", Kind::Coord(CoordFmt::PlusCode), "olc google plus code address", true);
    b.add("geohash", "geohash", Kind::Coord(CoordFmt::Geohash), "geohash location hash", true);
}

pub type LatLon = (f64, f64);

pub fn accepts(s: &str) -> bool {
    s.len() <= 80 && s.chars().all(|c| c.is_ascii_alphanumeric() || " .,;:+-−°'′\"″’”".contains(c))
}

impl CoordFmt {
    pub fn placeholder(self) -> &'static str {
        match self {
            CoordFmt::PlusCode => "e.g. 9G2GFG2M+2C",
            CoordFmt::Geohash => "e.g. u8vxn2",
            _ => "e.g. 50.4501, 30.5234 or 50°27′N 30°31′E",
        }
    }

    /// None while the second half is still missing.
    pub fn parse(self, s: &str) -> Result<Option<LatLon>, String> {
        let s = s.trim();
        if s.is_empty() {
            return Ok(None);
        }
        let p = match self {
            CoordFmt::PlusCode => plus_decode(s)?,
            CoordFmt::Geohash => geohash_decode(s)?,
            _ => match degrees(s)? {
                Some(p) => p,
                None => return Ok(None),
            },
        };
        if p.0.abs() > 90.0 || p.1.abs() > 180.0 {
            return Err("Latitude must be within ±90° and longitude within ±180°".into());
        }
        Ok(Some(p))
    }

    pub fn format(self, (lat, lon): LatLon) -> String {
        let hemi = |v: f64, pos: char, neg: char| if v < 0.0 { neg } else { pos };
        match self {
            CoordFmt::Decimal => format!("{lat:.6}, {lon:.6}"),
            CoordFmt::Dms => {
                let one = |v: f64, p, n| {
                    // Round once at the shown precision so 59.95″ becomes the next minute.
                    let tenths = (v.abs() * 36000.0).round() as i64;
                    format!("{}°{:02}′{:04.1}″{}", tenths / 36000, tenths / 600 % 60, (tenths % 600) as f64 / 10.0, hemi(v, p, n))
                };
                format!("{} {}", one(lat, 'N', 'S'), one(lon, 'E', 'W'))
            }
            CoordFmt::Ddm => {
                let one = |v: f64, p, n| {
                    let thou = (v.abs() * 60000.0).round() as i64;
                    format!("{}°{:06.3}′{}", thou / 60000, (thou % 60000) as f64 / 1000.0, hemi(v, p, n))
                };
                format!("{} {}", one(lat, 'N', 'S'), one(lon, 'E', 'W'))
            }
            CoordFmt::PlusCode => plus_encode(lat, lon),
            CoordFmt::Geohash => geohash_encode(lat, lon, 9),
        }
    }
}

/// Any degree notation: "50.45, 30.52", "50.45 N 30.52 E", "N 50° 27.006′ E 30° 31.404′",
/// "50°27′00.4″N 30°31′24.2″E", "-33 52 04 151 12 36".
fn degrees(s: &str) -> Result<Option<LatLon>, String> {
    // Split into groups of numbers; a hemisphere letter or a comma ends a group.
    let mut groups: Vec<(Vec<f64>, Option<char>)> = Vec::new();
    let mut cur: Vec<f64> = Vec::new();
    let mut pending: Option<char> = None;
    let s = s.replace('−', "-");
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c.is_ascii_digit() || c == '.' || (c == '-' && chars.peek().is_some_and(|n| n.is_ascii_digit() || *n == '.')) {
            let mut n = c.to_string();
            while let Some(&d) = chars.peek().filter(|d| d.is_ascii_digit() || **d == '.') {
                n.push(d);
                chars.next();
            }
            cur.push(n.parse().map_err(|_| format!("Can't read the number {n}"))?);
        } else if "NSEWnsew".contains(c) {
            let h = c.to_ascii_uppercase();
            if cur.is_empty() {
                pending = Some(h);
            } else if let Some(p) = pending.replace(h) {
                // "N 50 27 E 30 31": letters come first, so this one starts the next group.
                groups.push((std::mem::take(&mut cur), Some(p)));
            } else {
                pending = None;
                groups.push((std::mem::take(&mut cur), Some(h)));
            }
        } else if (c == ',' || c == ';') && !cur.is_empty() {
            groups.push((std::mem::take(&mut cur), pending.take()));
        } else if c.is_ascii_alphabetic() {
            return Err(format!("Unexpected letter {c}; use N, S, E or W"));
        }
    }
    if !cur.is_empty() {
        groups.push((cur, pending));
    }
    // No separators at all: split the numbers evenly ("50 27 0 30 31 24").
    if groups.len() == 1 && groups[0].1.is_none() && matches!(groups[0].0.len(), 2 | 4 | 6) {
        let (nums, _) = groups.remove(0);
        let half = nums.len() / 2;
        groups = vec![(nums[..half].to_vec(), None), (nums[half..].to_vec(), None)];
    }
    match groups.len() {
        0 | 1 => return Ok(None),
        2 => {}
        _ => return Err("Expected two coordinates: latitude and longitude".into()),
    }
    let mut vals = Vec::new();
    for (nums, h) in &groups {
        if nums.len() > 3 {
            return Err("Use degrees, minutes and seconds at most".into());
        }
        if nums.iter().skip(1).any(|v| *v < 0.0 || *v >= 60.0) {
            return Err("Minutes and seconds must be from 0 to 59.99".into());
        }
        let neg = nums[0] < 0.0 || matches!(h, Some('S' | 'W'));
        let v = nums[0].abs() + nums.get(1).unwrap_or(&0.0) / 60.0 + nums.get(2).unwrap_or(&0.0) / 3600.0;
        vals.push((if neg { -v } else { v }, *h));
    }
    // Longitude written first ("30.5 E, 50.4 N") gets swapped.
    let swap = matches!(vals[0].1, Some('E' | 'W')) || matches!(vals[1].1, Some('N' | 'S'));
    Ok(Some(if swap { (vals[1].0, vals[0].0) } else { (vals[0].0, vals[1].0) }))
}

const OLC: &[u8] = b"23456789CFGHJMPQRVWX";

/// 10-digit Open Location Code (about 14 × 14 m).
fn plus_encode(lat: f64, lon: f64) -> String {
    let mut lat = (lat.clamp(-90.0, 90.0) + 90.0).min(179.999_999_9);
    let mut lon = (lon + 180.0).rem_euclid(360.0);
    let mut code = String::new();
    let mut res = 20.0;
    for i in 0..5 {
        let (a, b) = ((lat / res).floor(), (lon / res).floor());
        code.push(OLC[a as usize] as char);
        code.push(OLC[b as usize] as char);
        lat -= a * res;
        lon -= b * res;
        res /= 20.0;
        if i == 3 {
            code.push('+');
        }
    }
    code
}

/// The center of a full plus code's area.
fn plus_decode(s: &str) -> Result<LatLon, String> {
    let code: String = s.trim().to_uppercase().chars().filter(|c| *c != '0').collect();
    let plus = code.find('+').ok_or("A plus code has a + after 8 characters, e.g. 9G2GFG2M+2C")?;
    if plus != 8 {
        return Err("Use a full plus code (8 characters before the +), not a short one".into());
    }
    let digits: Vec<u8> = code.bytes().filter(|b| *b != b'+').collect();
    if digits.len() < 2 || !digits.len().is_multiple_of(2) && digits.len() < 10 {
        return Err("Incomplete plus code".into());
    }
    let (mut lat, mut lon, mut res) = (0.0, 0.0, 20.0);
    for pair in digits.chunks(2).take(5) {
        let pos = |b: u8| OLC.iter().position(|o| *o == b).ok_or("Plus codes use 2–9 and CFGHJMPQRVWX".to_string());
        lat += pos(pair[0])? as f64 * res;
        lon += pos(*pair.get(1).unwrap_or(&b'2'))? as f64 * res;
        res /= 20.0;
    }
    let res = res * 20.0;
    Ok((lat - 90.0 + res / 2.0, lon - 180.0 + res / 2.0))
}

const B32: &[u8] = b"0123456789bcdefghjkmnpqrstuvwxyz";

fn geohash_encode(lat: f64, lon: f64, len: usize) -> String {
    let (mut la, mut lo) = ((-90.0, 90.0), (-180.0, 180.0));
    let (mut out, mut bits, mut ch, mut even) = (String::new(), 0, 0usize, true);
    while out.len() < len {
        let (r, v) = if even { (&mut lo, lon) } else { (&mut la, lat) };
        let mid = (r.0 + r.1) / 2.0;
        ch <<= 1;
        if v >= mid {
            ch |= 1;
            r.0 = mid;
        } else {
            r.1 = mid;
        }
        even = !even;
        bits += 1;
        if bits == 5 {
            out.push(B32[ch] as char);
            (bits, ch) = (0, 0);
        }
    }
    out
}

fn geohash_decode(s: &str) -> Result<LatLon, String> {
    let (mut la, mut lo) = ((-90.0f64, 90.0f64), (-180.0f64, 180.0f64));
    let mut even = true;
    for c in s.trim().to_lowercase().bytes() {
        let v = B32.iter().position(|b| *b == c).ok_or("Geohash uses 0–9 and b–z without a, i, l, o")?;
        for bit in (0..5).rev() {
            let r = if even { &mut lo } else { &mut la };
            let mid = (r.0 + r.1) / 2.0;
            if v >> bit & 1 == 1 { r.0 = mid } else { r.1 = mid }
            even = !even;
        }
    }
    Ok(((la.0 + la.1) / 2.0, (lo.0 + lo.1) / 2.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use CoordFmt::*;

    fn conv(a: CoordFmt, b: CoordFmt, s: &str) -> String {
        b.format(a.parse(s).unwrap().unwrap())
    }

    #[test]
    fn coordinates() {
        assert_eq!(conv(Decimal, Dms, "50.4501, 30.5234"), "50°27′00.4″N 30°31′24.2″E");
        assert_eq!(conv(Dms, Decimal, "50°27′00.4″N 30°31′24.2″E"), "50.450111, 30.523389");
        assert_eq!(conv(Dms, Decimal, "50 27 0.4 N, 30 31 24.2 E"), "50.450111, 30.523389");
        assert_eq!(conv(Ddm, Decimal, "N 50° 27.006′ E 30° 31.404′"), "50.450100, 30.523400");
        assert_eq!(conv(Decimal, Decimal, "30.5234 E 50.4501 N"), "50.450100, 30.523400");
        assert_eq!(conv(Decimal, Ddm, "-33.8688, 151.2093"), "33°52.128′S 151°12.558′E");
        assert_eq!(conv(Decimal, Decimal, "-33 52 7.7 151 12 33.5"), "-33.868806, 151.209306");
        // Plus code and geohash for a known spot (Google's office in Zurich: 8FVC9G8F+6X).
        assert_eq!(conv(Decimal, PlusCode, "47.365590, 8.524997"), "8FVC9G8F+6X");
        let back = PlusCode.parse("8FVC9G8F+6X").unwrap().unwrap();
        assert!((back.0 - 47.36559).abs() < 1e-4 && (back.1 - 8.524997).abs() < 1e-4);
        assert_eq!(conv(Decimal, Geohash, "57.64911, 10.40744"), "u4pruydqq");
        let g = Geohash.parse("u4pruydqq").unwrap().unwrap();
        assert!((g.0 - 57.64911).abs() < 1e-4 && (g.1 - 10.40744).abs() < 1e-4);
        assert_eq!(Decimal.parse("50.45"), Ok(None));
        assert!(Decimal.parse("95, 10").is_err());
        assert!(Dms.parse("50 70 0 N 30 0 0 E").is_err());
        assert!(PlusCode.parse("GFG2M+2C").is_err());
    }
}
