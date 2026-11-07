//! Time in cities around the world. Values are exact Unix seconds, like dates; each unit shows
//! and reads the wall-clock time of one place, daylight saving included. Zone rules come from
//! the system's tz database, or jiff's built-in copy where there is none (Windows).

use super::{Builder, Kind};
use crate::num::{int, R};
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use num_traits::ToPrimitive;

/// (name, IANA zone, search words). Index 0 is this computer's own zone.
pub const ZONES: &[(&str, &str, &str)] = &[
    ("Local time (this computer)", "", "here my time zone local now"),
    ("UTC", "UTC", "gmt zulu utc coordinated universal time z"),
    // Europe
    ("London", "Europe/London", "uk united kingdom england britain gmt bst"),
    ("Dublin", "Europe/Dublin", "ireland ist"),
    ("Lisbon", "Europe/Lisbon", "portugal wet"),
    ("Reykjavik", "Atlantic/Reykjavik", "iceland"),
    ("Paris", "Europe/Paris", "france cet cest"),
    ("Berlin", "Europe/Berlin", "germany deutschland cet cest munich frankfurt hamburg"),
    ("Madrid", "Europe/Madrid", "spain barcelona cet"),
    ("Rome", "Europe/Rome", "italy milan cet"),
    ("Amsterdam", "Europe/Amsterdam", "netherlands holland cet"),
    ("Brussels", "Europe/Brussels", "belgium cet"),
    ("Zurich", "Europe/Zurich", "switzerland geneva bern cet"),
    ("Vienna", "Europe/Vienna", "austria cet"),
    ("Prague", "Europe/Prague", "czech republic czechia cet"),
    ("Warsaw", "Europe/Warsaw", "poland krakow cet"),
    ("Budapest", "Europe/Budapest", "hungary cet"),
    ("Stockholm", "Europe/Stockholm", "sweden cet"),
    ("Oslo", "Europe/Oslo", "norway cet"),
    ("Copenhagen", "Europe/Copenhagen", "denmark cet"),
    ("Helsinki", "Europe/Helsinki", "finland eet"),
    ("Tallinn", "Europe/Tallinn", "estonia eet"),
    ("Riga", "Europe/Riga", "latvia eet"),
    ("Vilnius", "Europe/Vilnius", "lithuania eet"),
    ("Kyiv", "Europe/Kyiv", "kiev ukraine київ eet eest lviv odesa kharkiv dnipro"),
    ("Chisinau", "Europe/Chisinau", "moldova eet"),
    ("Bucharest", "Europe/Bucharest", "romania eet"),
    ("Sofia", "Europe/Sofia", "bulgaria eet"),
    ("Athens", "Europe/Athens", "greece eet"),
    ("Belgrade", "Europe/Belgrade", "serbia cet"),
    ("Zagreb", "Europe/Zagreb", "croatia cet"),
    ("Istanbul", "Europe/Istanbul", "turkey türkiye ankara trt"),
    ("Minsk", "Europe/Minsk", "belarus"),
    ("Moscow", "Europe/Moscow", "russia msk saint petersburg"),
    // Middle East & Africa
    ("Tbilisi", "Asia/Tbilisi", "georgia"),
    ("Yerevan", "Asia/Yerevan", "armenia"),
    ("Baku", "Asia/Baku", "azerbaijan"),
    ("Jerusalem", "Asia/Jerusalem", "israel tel aviv"),
    ("Beirut", "Asia/Beirut", "lebanon"),
    ("Amman", "Asia/Amman", "jordan"),
    ("Baghdad", "Asia/Baghdad", "iraq"),
    ("Riyadh", "Asia/Riyadh", "saudi arabia ast jeddah mecca"),
    ("Doha", "Asia/Qatar", "qatar"),
    ("Dubai", "Asia/Dubai", "uae united arab emirates abu dhabi gst"),
    ("Tehran", "Asia/Tehran", "iran"),
    ("Cairo", "Africa/Cairo", "egypt eet"),
    ("Casablanca", "Africa/Casablanca", "morocco"),
    ("Lagos", "Africa/Lagos", "nigeria wat"),
    ("Accra", "Africa/Accra", "ghana"),
    ("Nairobi", "Africa/Nairobi", "kenya eat"),
    ("Addis Ababa", "Africa/Addis_Ababa", "ethiopia eat"),
    ("Johannesburg", "Africa/Johannesburg", "south africa sast cape town pretoria"),
    ("Kinshasa", "Africa/Kinshasa", "congo drc"),
    // Asia
    ("Karachi", "Asia/Karachi", "pakistan pkt lahore islamabad"),
    ("Tashkent", "Asia/Tashkent", "uzbekistan"),
    ("Almaty", "Asia/Almaty", "kazakhstan astana"),
    ("Kabul", "Asia/Kabul", "afghanistan"),
    ("New Delhi", "Asia/Kolkata", "india ist mumbai bangalore bengaluru kolkata chennai"),
    ("Kathmandu", "Asia/Kathmandu", "nepal"),
    ("Dhaka", "Asia/Dhaka", "bangladesh"),
    ("Colombo", "Asia/Colombo", "sri lanka"),
    ("Yangon", "Asia/Yangon", "myanmar burma rangoon"),
    ("Bangkok", "Asia/Bangkok", "thailand ict"),
    ("Hanoi", "Asia/Ho_Chi_Minh", "vietnam ho chi minh saigon"),
    ("Jakarta", "Asia/Jakarta", "indonesia wib"),
    ("Kuala Lumpur", "Asia/Kuala_Lumpur", "malaysia"),
    ("Singapore", "Asia/Singapore", "sgt"),
    ("Manila", "Asia/Manila", "philippines pht"),
    ("Beijing", "Asia/Shanghai", "china shanghai shenzhen guangzhou cst"),
    ("Hong Kong", "Asia/Hong_Kong", "hkt"),
    ("Taipei", "Asia/Taipei", "taiwan"),
    ("Seoul", "Asia/Seoul", "south korea kst"),
    ("Tokyo", "Asia/Tokyo", "japan jst osaka kyoto"),
    ("Ulaanbaatar", "Asia/Ulaanbaatar", "mongolia"),
    ("Vladivostok", "Asia/Vladivostok", "russia far east"),
    ("Yekaterinburg", "Asia/Yekaterinburg", "russia urals"),
    ("Novosibirsk", "Asia/Novosibirsk", "russia siberia"),
    // Oceania
    ("Perth", "Australia/Perth", "australia awst western"),
    ("Adelaide", "Australia/Adelaide", "australia acst south"),
    ("Darwin", "Australia/Darwin", "australia northern territory"),
    ("Brisbane", "Australia/Brisbane", "australia queensland aest"),
    ("Sydney", "Australia/Sydney", "australia aest aedt new south wales canberra"),
    ("Melbourne", "Australia/Melbourne", "australia victoria aest"),
    ("Hobart", "Australia/Hobart", "australia tasmania"),
    ("Auckland", "Pacific/Auckland", "new zealand nzst nzdt wellington"),
    ("Fiji", "Pacific/Fiji", "suva"),
    ("Honolulu", "Pacific/Honolulu", "hawaii hst usa"),
    // Americas
    ("Anchorage", "America/Anchorage", "alaska akst usa"),
    ("Los Angeles", "America/Los_Angeles", "usa california pacific pst pdt san francisco seattle"),
    ("Vancouver", "America/Vancouver", "canada pacific pst british columbia"),
    ("Phoenix", "America/Phoenix", "usa arizona mst"),
    ("Denver", "America/Denver", "usa mountain mst mdt colorado"),
    ("Chicago", "America/Chicago", "usa central cst cdt texas dallas houston"),
    ("Mexico City", "America/Mexico_City", "mexico cst"),
    ("New York", "America/New_York", "usa eastern est edt nyc washington boston miami atlanta"),
    ("Toronto", "America/Toronto", "canada eastern est montreal ottawa"),
    ("Havana", "America/Havana", "cuba"),
    ("Panama", "America/Panama", "panama"),
    ("Bogotá", "America/Bogota", "colombia bogota"),
    ("Lima", "America/Lima", "peru"),
    ("Caracas", "America/Caracas", "venezuela"),
    ("Halifax", "America/Halifax", "canada atlantic ast"),
    ("Santiago", "America/Santiago", "chile"),
    ("La Paz", "America/La_Paz", "bolivia"),
    ("St. John's", "America/St_Johns", "canada newfoundland nst"),
    ("Buenos Aires", "America/Argentina/Buenos_Aires", "argentina art"),
    ("São Paulo", "America/Sao_Paulo", "brazil brt sao paulo rio de janeiro brasilia"),
    ("Montevideo", "America/Montevideo", "uruguay"),
];

pub fn register(b: &mut Builder) {
    b.category("timezone", "Time Zones", false);
    for (i, &(name, iana, kw)) in ZONES.iter().enumerate() {
        let label = if i < 2 { name.to_string() } else { format!("{name} time") };
        let sym = if i == 0 { "local" } else if i == 1 { "UTC" } else { name };
        b.add(&label, sym, Kind::Zone(i), &format!("{iana} {kw} clock world time zone"), i < 2);
    }
}

fn tz(i: usize) -> Result<TimeZone, String> {
    match ZONES.get(i) {
        Some((_, "", _)) => Ok(TimeZone::system()),
        Some((name, iana, _)) => TimeZone::get(iana).map_err(|_| format!("No time zone data for {name}")),
        None => Err("Unknown time zone".into()),
    }
}

/// Characters a time can be typed with ("2026-11-07 3:30 pm", "now").
pub fn accepts(s: &str) -> bool {
    s.len() <= 40 && s.chars().all(|c| c.is_ascii_alphanumeric() || " -:./()+,".contains(c))
}

/// "Sat 2026-11-07 15:00 (UTC+2)". Reading it back ignores the weekday and the brackets.
pub fn format(i: usize, v: &R) -> Result<String, String> {
    let secs = v.floor().to_integer().to_i64().ok_or("Date out of range")?;
    let z = Timestamp::from_second(secs).map_err(|_| "Date out of range")?.to_zoned(tz(i)?);
    let time = if z.second() == 0 { z.strftime("%H:%M").to_string() } else { z.strftime("%H:%M:%S").to_string() };
    Ok(format!("{} {time} ({})", z.strftime("%a %Y-%m-%d"), offset_text(z.offset().seconds())))
}

/// "UTC+2", "UTC−3:30", "UTC".
pub fn offset_text(secs: i32) -> String {
    let (sign, s) = if secs < 0 { ('−', -secs) } else { ('+', secs) };
    match (s / 3600, s % 3600 / 60) {
        (0, 0) => "UTC".into(),
        (h, 0) => format!("UTC{sign}{h}"),
        (h, m) => format!("UTC{sign}{h}:{m:02}"),
    }
}

/// A zone's UTC offset at a moment (Unix seconds), in seconds.
pub fn offset_at(i: usize, v: &R) -> Option<i32> {
    let ts = Timestamp::from_second(v.floor().to_integer().to_i64()?).ok()?;
    Some(ts.to_zoned(tz(i).ok()?).offset().seconds())
}

/// "7 h ahead of New York", "5:30 h behind Tokyo", "same time as Paris".
pub fn gap(here: i32, there: i32, there_name: &str) -> String {
    let d = here - there;
    let (h, m) = (d.abs() / 3600, d.abs() % 3600 / 60);
    let amount = if m == 0 { format!("{h} h") } else { format!("{h}:{m:02} h") };
    match d {
        0 => format!("same time as {there_name}"),
        d if d > 0 => format!("{amount} ahead of {there_name}"),
        _ => format!("{amount} behind {there_name}"),
    }
}

/// The time right now in a zone, as typed: "15:04".
pub fn now_text(i: usize) -> String {
    tz(i).map(|t| Timestamp::now().to_zoned(t).strftime("%H:%M").to_string()).unwrap_or_else(|_| "12:00".into())
}

/// "15:00", "3pm", "3:30 pm", "2026-11-07", "2026-11-07 09:30", "07.11.2026 9:30", "now".
/// A time alone means today in that place. None while still being typed.
pub fn parse(i: usize, s: &str) -> Result<Option<R>, String> {
    let tz = tz(i)?;
    // Drop what `format` adds: the "(UTC+2)" note and a leading weekday.
    let s = s.split('(').next().unwrap_or("").trim().to_lowercase();
    if s == "now" {
        return Ok(Some(int(Timestamp::now().as_second())));
    }
    let s = s.trim_start_matches(|c: char| c.is_ascii_alphabetic()).trim().to_string();
    if s.is_empty() {
        return Ok(None);
    }
    let (mut date_part, mut time_part) = ("", s.as_str());
    if let Some((d, t)) = s.split_once([' ', 't']).filter(|(d, _)| d.contains(['-', '.', '/'])) {
        (date_part, time_part) = (d, t.trim());
    } else if s.contains(['-', '.', '/']) && !s.contains(':') {
        (date_part, time_part) = (&s, "");
    }
    let (y, mo, d) = if date_part.is_empty() {
        let today = Timestamp::now().to_zoned(tz.clone());
        (today.year() as i64, today.month() as i64, today.day() as i64)
    } else {
        match parse_date(date_part)? {
            Some(d) => d,
            None => return Ok(None),
        }
    };
    let Some((h, mi, sec)) = parse_time(time_part)? else { return Ok(None) };
    let dt = DateTime::new(y as i16, mo as i8, d as i8, h as i8, mi as i8, sec as i8, 0).map_err(|_| "Not a real date or time".to_string())?;
    let z = tz.to_ambiguous_zoned(dt).compatible().map_err(|_| "Date out of range".to_string())?;
    Ok(Some(int(z.timestamp().as_second())))
}

/// "2026-11-07", "2026/11/7" or "07.11.2026". None while incomplete.
fn parse_date(s: &str) -> Result<Option<(i64, i64, i64)>, String> {
    let nums: Vec<&str> = s.split(['-', '.', '/']).collect();
    if nums.len() < 3 || nums.iter().any(|n| n.is_empty()) {
        return Ok(None);
    }
    let n: Vec<i64> = nums.iter().map(|x| x.parse::<i64>().map_err(|_| "Dates look like 2026-11-07".to_string())).collect::<Result<_, _>>()?;
    let (y, m, d) = if nums[0].len() == 4 { (n[0], n[1], n[2]) } else { (n[2], n[1], n[0]) };
    if !(1..=9999).contains(&y) || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err("Not a real date".into());
    }
    Ok(Some((y, m, d)))
}

/// "15", "15:30", "15:30:10", "3pm", "3:30 am". Empty means midnight. None while incomplete.
fn parse_time(s: &str) -> Result<Option<(i64, i64, i64)>, String> {
    let s = s.replace(' ', "");
    if s.is_empty() {
        return Ok(Some((0, 0, 0)));
    }
    let (clock, ampm) = match s.strip_suffix("am").or(s.strip_suffix("a")) {
        Some(c) => (c, Some(false)),
        None => match s.strip_suffix("pm").or(s.strip_suffix("p")) {
            Some(c) => (c, Some(true)),
            None => (s.as_str(), None),
        },
    };
    let parts: Vec<&str> = clock.split(':').collect();
    if parts.iter().any(|p| p.is_empty()) || parts.len() > 3 {
        return if parts.len() <= 3 { Ok(None) } else { Err("Times look like 15:30 or 3:30 pm".into()) };
    }
    let n: Vec<i64> = parts.iter().map(|p| p.parse::<i64>().map_err(|_| "Times look like 15:30 or 3:30 pm".to_string())).collect::<Result<_, _>>()?;
    let (mut h, mi, sec) = (n[0], n.get(1).copied().unwrap_or(0), n.get(2).copied().unwrap_or(0));
    match ampm {
        Some(_) if !(1..=12).contains(&h) => return Err("With am/pm the hour is 1–12".into()),
        Some(pm) => h = h % 12 + if pm { 12 } else { 0 },
        None => {}
    }
    if h > 23 || mi > 59 || sec > 59 {
        return Err("Not a real time of day".into());
    }
    Ok(Some((h, mi, sec)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx(name: &str) -> usize {
        ZONES.iter().position(|z| z.0 == name).unwrap()
    }

    #[test]
    fn zones_resolve_and_convert() {
        for (i, z) in ZONES.iter().enumerate() {
            assert!(tz(i).is_ok(), "{}", z.1);
        }
        // Winter: Kyiv is UTC+2, New York UTC−5.
        let v = parse(idx("Kyiv"), "2026-01-15 15:00").unwrap().unwrap();
        assert_eq!(format(idx("New York"), &v).unwrap(), "Thu 2026-01-15 08:00 (UTC−5)");
        // Summer time moves both.
        let v = parse(idx("London"), "2026-07-01 3pm").unwrap().unwrap();
        assert_eq!(format(idx("Tokyo"), &v).unwrap(), "Wed 2026-07-01 23:00 (UTC+9)");
        assert_eq!(format(idx("New Delhi"), &v).unwrap(), "Wed 2026-07-01 19:30 (UTC+5:30)");
        // Output reads back as the same moment.
        let shown = format(idx("Sydney"), &v).unwrap();
        assert_eq!(parse(idx("Sydney"), &shown).unwrap().unwrap(), v);
        assert_eq!(parse(idx("UTC"), "07.11.2026 9:30").unwrap(), parse(idx("UTC"), "2026-11-07 09:30").unwrap());
        assert_eq!(parse(idx("UTC"), "15:").unwrap(), None);
        assert!(parse(idx("UTC"), "25:00").is_err());
        assert!(parse(idx("UTC"), "13pm").is_err());
    }
}
