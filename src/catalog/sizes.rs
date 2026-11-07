//! Size charts (rings, clothing, bras, paper): each category is a table, each unit one column.
//! The value is a row number, so converting looks the typed size up in its column and shows
//! the same row in another. Typed numbers that aren't in the chart snap to the nearest row.

use super::{Builder, Kind};
use crate::num::{int, R};
use num_traits::ToPrimitive;
use std::sync::OnceLock;

pub struct Table {
    pub id: &'static str,
    pub name: &'static str,
    /// (unit name, symbol, search words)
    pub cols: &'static [(&'static str, &'static str, &'static str)],
    /// Cells; "|" separates extra spellings accepted when typing ("Letter|US Letter").
    pub rows: Vec<Vec<String>>,
}

pub fn tables() -> &'static [Table] {
    static T: OnceLock<Vec<Table>> = OnceLock::new();
    T.get_or_init(|| vec![rings(), womens(), mens_suits(), mens_shirts(), bras(), paper()])
}

pub fn register(b: &mut Builder) {
    for (t, table) in tables().iter().enumerate() {
        b.category(table.id, table.name, false);
        for (c, &(name, sym, kw)) in table.cols.iter().enumerate() {
            b.add(name, sym, Kind::Size(t as u8, c as u8), &format!("{kw} size chart"), true);
        }
    }
}

/// "6.5" not "6.50"; "215.9" not "215.90".
fn num(v: f64, places: usize) -> String {
    let s = format!("{v:.places$}");
    if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s }
}

fn rings() -> Table {
    // US size 0 is 11.63 mm across, each size adds 0.8128 mm (1/32 inch).
    // shortcut: UK/JP columns follow common jeweller charts, which differ by up to half a size.
    const JP: [u32; 21] = [4, 5, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17, 18, 19, 20, 22, 23, 24, 25, 26, 27];
    // Quarter US sizes: each step is about half a UK letter, so every letter appears.
    let rows = (0..=40)
        .map(|i| {
            let us = 3.0 + i as f64 * 0.25;
            let dia = 11.63 + 0.8128 * us;
            let circ = std::f64::consts::PI * dia;
            let uk = ((circ - 37.43) / 1.25 * 2.0).round() / 2.0;
            let letter = |n: f64| {
                let n = n as usize;
                if n < 26 { ((b'A' + n as u8) as char).to_string() } else { format!("Z+{}", n - 25) }
            };
            let uk = if uk.fract() > 0.0 { format!("{}½|{}1/2|{}.5", letter(uk), letter(uk), letter(uk)) } else { letter(uk) };
            let half = i as f64 / 2.0;
            let lo = JP[(half as usize).min(20)] as f64;
            let jp = lo + (JP[(half as usize + 1).min(20)] as f64 - lo) * half.fract();
            vec![num(us, 2), uk, format!("{:.0}", circ), format!("{:.0}", jp.round()), format!("{dia:.1}")]
        })
        .collect();
    Table {
        id: "ring",
        name: "Ring Size",
        cols: &[
            ("US / Canada ring size", "US", "american ring"),
            ("UK / Australia ring size", "UK", "british letter ring"),
            ("EU / ISO ring size (circumference mm)", "EU", "europe iso circumference ring"),
            ("Japan ring size", "JP", "japanese ring"),
            ("ring inner diameter (mm)", "⌀ mm", "diameter ring"),
        ],
        rows,
    }
}

fn table(cols: &[&[&str]]) -> Vec<Vec<String>> {
    (0..cols[0].len()).map(|r| cols.iter().map(|c| c[r].to_string()).collect()).collect()
}

fn womens() -> Table {
    Table {
        id: "womens",
        name: "Women's Clothing",
        cols: &[
            ("women's letter size", "XS–XL", "women dress letter small medium large"),
            ("US women's size", "US", "american women dress"),
            ("UK women's size", "UK", "british women dress"),
            ("EU / German women's size", "EU", "europe germany women dress"),
            ("French women's size", "FR", "france women dress"),
            ("Italian women's size", "IT", "italy women dress"),
            ("Japanese women's size", "JP", "japan women dress"),
        ],
        rows: table(&[
            &["XS", "XS", "S", "S", "M", "M", "L", "L", "XL", "XL", "XXL|2XL"],
            &["0", "2", "4", "6", "8", "10", "12", "14", "16", "18", "20"],
            &["4", "6", "8", "10", "12", "14", "16", "18", "20", "22", "24"],
            &["30", "32", "34", "36", "38", "40", "42", "44", "46", "48", "50"],
            &["32", "34", "36", "38", "40", "42", "44", "46", "48", "50", "52"],
            &["36", "38", "40", "42", "44", "46", "48", "50", "52", "54", "56"],
            &["3", "5", "7", "9", "11", "13", "15", "17", "19", "21", "23"],
        ]),
    }
}

fn mens_suits() -> Table {
    Table {
        id: "menssuit",
        name: "Men's Suits & Jackets",
        cols: &[
            ("men's jacket letter size", "S–XL", "men suit jacket coat letter small medium large"),
            ("US / UK jacket size (chest in)", "US/UK", "american british men suit jacket blazer chest"),
            ("EU jacket size", "EU", "europe men suit jacket blazer"),
            ("chest (cm)", "chest cm", "men chest girth"),
        ],
        rows: table(&[
            &["XS", "S", "S", "M", "L", "L", "XL", "XXL|2XL", "XXL|2XL", "3XL|XXXL", "4XL"],
            &["34", "36", "38", "40", "42", "44", "46", "48", "50", "52", "54"],
            &["44", "46", "48", "50", "52", "54", "56", "58", "60", "62", "64"],
            &["86", "91", "97", "102", "107", "112", "117", "122", "127", "132", "137"],
        ]),
    }
}

fn mens_shirts() -> Table {
    Table {
        id: "mensshirt",
        name: "Men's Shirts",
        cols: &[
            ("men's shirt letter size", "S–XL", "men shirt letter small medium large"),
            ("US / UK collar size (in)", "collar in", "american british men shirt neck collar"),
            ("EU collar size (cm)", "collar cm", "europe men shirt neck collar"),
        ],
        rows: table(&[
            &["S", "S", "M", "M", "L", "L", "XL", "XL", "XXL|2XL", "XXL|2XL"],
            &["14", "14.5", "15", "15.5", "16", "16.5", "17", "17.5", "18", "18.5"],
            &["36", "37", "38", "39", "41", "42", "43", "44", "45", "47"],
        ]),
    }
}

fn bras() -> Table {
    const US: [&str; 10] = ["A", "B", "C", "D", "DD|E", "DDD|F", "G", "H", "I", "J"];
    const UK: [&str; 10] = ["A", "B", "C", "D", "DD", "E", "F", "FF", "G", "GG"];
    const EU: [&str; 10] = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];
    let mut rows = Vec::new();
    for band in (28..=44).step_by(2) {
        let eu = 60 + (band - 28) / 2 * 5;
        for c in 0..10 {
            let with = |b: i32, cups: &str| cups.split('|').map(|cup| format!("{b}{cup}")).collect::<Vec<_>>().join("|");
            rows.push(vec![with(band, US[c]), with(band, UK[c]), with(eu, EU[c]), with(eu + 15, EU[c]), with(band - 22, UK[c])]);
        }
    }
    Table {
        id: "bra",
        name: "Bra Size",
        cols: &[
            ("US bra size", "US", "american bra band cup"),
            ("UK bra size", "UK", "british bra band cup"),
            ("EU bra size", "EU", "europe bra band cup"),
            ("French / Belgian bra size", "FR", "france belgium bra band cup"),
            ("Australian bra size", "AU", "australia new zealand bra band cup"),
        ],
        rows,
    }
}

fn paper() -> Table {
    const SIZES: &[(&str, f64, f64)] = &[
        ("A0", 841.0, 1189.0), ("A1", 594.0, 841.0), ("A2", 420.0, 594.0), ("A3", 297.0, 420.0), ("A4", 210.0, 297.0),
        ("A5", 148.0, 210.0), ("A6", 105.0, 148.0), ("A7", 74.0, 105.0), ("A8", 52.0, 74.0),
        ("B0", 1000.0, 1414.0), ("B1", 707.0, 1000.0), ("B2", 500.0, 707.0), ("B3", 353.0, 500.0), ("B4", 250.0, 353.0),
        ("B5", 176.0, 250.0), ("B6", 125.0, 176.0),
        ("C4 envelope|C4", 229.0, 324.0), ("C5 envelope|C5", 162.0, 229.0), ("C6 envelope|C6", 114.0, 162.0), ("DL envelope|DL", 110.0, 220.0),
        ("US Letter|Letter|ANSI A", 215.9, 279.4), ("US Legal|Legal", 215.9, 355.6), ("Tabloid|Ledger|ANSI B", 279.4, 431.8),
        ("Executive", 184.15, 266.7), ("Half Letter|Statement", 139.7, 215.9),
        ("Photo 4×6 in|4x6|10x15", 101.6, 152.4), ("Photo 5×7 in|5x7|13x18", 127.0, 177.8), ("Photo 8×10 in|8x10", 203.2, 254.0),
        ("Business card (EU)", 85.0, 55.0), ("Business card (US)", 88.9, 50.8), ("ID / bank card|credit card|ID-1", 85.6, 53.98),
    ];
    let px = |mm: f64, dpi: f64| format!("{:.0}", mm / 25.4 * dpi);
    let rows = SIZES
        .iter()
        .map(|&(name, w, h)| {
            let mut row = vec![name.to_string(), format!("{} × {} mm", num(w, 1), num(h, 1)), format!("{} × {} in", num(w / 25.4, 2), num(h / 25.4, 2))];
            for dpi in [300.0, 150.0, 96.0, 72.0] {
                row.push(format!("{} × {} px", px(w, dpi), px(h, dpi)));
            }
            row
        })
        .collect();
    Table {
        id: "paper",
        name: "Paper Sizes",
        cols: &[
            ("paper size name", "name", "a4 a3 a5 letter legal tabloid envelope photo card iso paper format"),
            ("paper size in millimeters", "mm", "paper dimensions metric"),
            ("paper size in inches", "in", "paper dimensions imperial"),
            ("paper size in pixels at 300 dpi", "px 300dpi", "print pixels resolution"),
            ("paper size in pixels at 150 dpi", "px 150dpi", "print pixels resolution"),
            ("paper size in pixels at 96 dpi", "px 96dpi", "screen pixels resolution"),
            ("paper size in pixels at 72 dpi", "px 72dpi", "screen pixels resolution"),
        ],
        rows,
    }
}

fn norm(s: &str) -> String {
    s.to_lowercase().replace(['×', '*'], "x").replace("½", "1/2").chars().filter(|c| !c.is_whitespace()).collect()
}

fn numbers(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.')).filter_map(|p| p.parse().ok()).collect()
}

/// Row number of a typed size. Exact spellings first, then the nearest number.
pub fn parse(t: u8, c: u8, s: &str) -> Result<Option<R>, String> {
    let table = &tables()[t as usize];
    let c = c as usize;
    let want = norm(s);
    if want.is_empty() {
        return Ok(None);
    }
    if let Some(r) = table.rows.iter().position(|row| row[c].split('|').any(|alt| norm(alt) == want)) {
        return Ok(Some(int(r as i64)));
    }
    let typed = numbers(s);
    let has_letters = s.chars().any(|ch| ch.is_alphabetic() && !"xXmminpd".contains(ch));
    if !typed.is_empty() && !has_letters {
        let sorted = |mut v: Vec<f64>| {
            v.sort_by(f64::total_cmp);
            v
        };
        let typed = sorted(typed);
        let best = table
            .rows
            .iter()
            .enumerate()
            .filter_map(|(r, row)| {
                let cell = sorted(numbers(row[c].split('|').next().unwrap_or("")));
                (cell.len() == typed.len()).then(|| (r, cell.iter().zip(&typed).map(|(a, b)| (a - b).abs() / a.abs().max(1.0)).sum::<f64>()))
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));
        if let Some((r, _)) = best.filter(|b| b.1 < 0.08) {
            return Ok(Some(int(r as i64)));
        }
    }
    let first = table.rows.first().map(|r| r[c].split('|').next().unwrap_or("")).unwrap_or("");
    Err(format!("Not in the {} chart — e.g. {first}", table.name.to_lowercase()))
}

pub fn format(t: u8, c: u8, v: &R) -> Result<String, String> {
    let table = &tables()[t as usize];
    let row = v.round().to_integer().to_usize().and_then(|r| table.rows.get(r)).ok_or("No matching size in this chart")?;
    Ok(row[c as usize].split('|').next().unwrap_or("").to_string())
}

pub fn example(t: u8, c: u8) -> String {
    let table = &tables()[t as usize];
    let row = match table.id {
        "paper" => 4,
        "bra" => 24,
        _ => table.rows.len() / 2,
    };
    table.rows[row.min(table.rows.len() - 1)][c as usize].split('|').next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conv(t: &str, from: usize, to: usize, s: &str) -> String {
        let i = tables().iter().position(|x| x.id == t).unwrap() as u8;
        format(i, to as u8, &parse(i, from as u8, s).unwrap().unwrap()).unwrap()
    }

    #[test]
    fn charts() {
        assert_eq!(conv("paper", 0, 1, "a4"), "210 × 297 mm");
        assert_eq!(conv("paper", 0, 2, "Letter"), "8.5 × 11 in");
        assert_eq!(conv("paper", 0, 3, "A4"), "2480 × 3508 px");
        assert_eq!(conv("paper", 1, 0, "297x210"), "A4");
        assert_eq!(conv("ring", 0, 1, "6"), "L½");
        assert_eq!(conv("ring", 1, 0, "L 1/2"), "6");
        assert_eq!(conv("ring", 1, 0, "l"), "5.75");
        assert_eq!(conv("ring", 0, 2, "7"), "54");
        assert_eq!(conv("ring", 0, 3, "7"), "14");
        assert_eq!(conv("ring", 4, 0, "17.3"), "7");
        assert_eq!(conv("womens", 1, 3, "8"), "38");
        assert_eq!(conv("womens", 0, 1, "m"), "8");
        assert_eq!(conv("bra", 0, 2, "34C"), "75C");
        assert_eq!(conv("bra", 1, 0, "34e"), "34DDD");
        assert_eq!(conv("mensshirt", 1, 2, "15.5"), "39");
        let a4 = tables().iter().position(|x| x.id == "paper").unwrap() as u8;
        assert!(parse(a4, 0, "A42").is_err());
        assert!(parse(a4, 1, "100 x 100").is_err());
        for (t, table) in tables().iter().enumerate() {
            assert!(table.rows.iter().all(|r| r.len() == table.cols.len()), "{}", table.id);
            for c in 0..table.cols.len() {
                assert!(parse(t as u8, c as u8, &example(t as u8, c as u8)).is_ok(), "{} {c}", table.id);
            }
        }
    }
}
