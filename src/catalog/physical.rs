//! Physical quantities. Each factor is an exact expression converting one unit into the
//! category's SI base unit. Names available in expressions: see `constant` in `mod.rs`.

use super::numbers::Frac;
use super::scale::Scale;
use super::time::{DateFmt, DurFmt};
use super::{Builder, Kind};

pub fn register(b: &mut Builder) {
    b.category("length", "Length", false);
    b.si("meter", "m", "1", false, &["k", "c", "m", "µ", "n", "d"], "metre");
    b.linear(&[
        ("inch", "in", "inch", "\" inches"),
        ("foot", "ft", "ft", "' feet"),
        ("yard", "yd", "yd", "yards"),
        ("mile", "mi", "mi", "miles statute"),
        ("nautical mile", "nmi", "nmi", "nm sea"),
        ("thou", "mil", "inch/1000", "thousandth inch"),
        ("hand", "hh", "4*inch", "horse"),
        ("link", "li", "66*ft/100", "surveyor gunter"),
        ("rod", "rd", "16.5*ft", "perch pole"),
        ("chain", "ch", "66*ft", "gunter"),
        ("furlong", "fur", "660*ft", ""),
        ("league", "lea", "3*mi", ""),
        ("fathom", "ftm", "6*ft", "depth"),
        ("cable", "cbl", "185.2", "nautical"),
        ("US survey foot", "ft(US)", "1200/3937", ""),
        ("angstrom", "Å", "1e-10", "ångström"),
        ("fermi", "fm(fermi)", "1e-15", "nuclear"),
        ("bohr radius", "a₀", "5.29177210903e-11", "atomic"),
        ("planck length", "lP", "1.616255e-35", ""),
        ("light-second", "ls", "c", "light second"),
        ("light-minute", "lmin", "c*60", "light minute"),
        ("light-hour", "lh", "c*3600", "light hour"),
        ("light-day", "ld", "c*day", "light day"),
        ("light-year", "ly", "c*365.25*day", "light year"),
        ("astronomical unit", "au", "au", "sun earth distance"),
        ("parsec", "pc", "648000/pi*au", ""),
        ("kiloparsec", "kpc", "648000/pi*au*1e3", ""),
        ("megaparsec", "Mpc", "648000/pi*au*1e6", ""),
        ("earth radius", "R⊕", "6.3781e6", "equatorial"),
        ("point (typographic)", "pt", "inch/72", "font dtp"),
        ("pica", "pc(pica)", "inch/6", "typography"),
        ("pixel (96 dpi)", "px", "inch/96", "css screen"),
        ("twip", "twip", "inch/1440", "typography"),
        ("cubit", "cbt", "18*inch", "ancient"),
        ("span", "span", "9*inch", "ancient"),
        ("ell", "ell", "45*inch", ""),
        ("barleycorn", "barleycorn", "inch/3", ""),
        ("verst", "verst", "1066.8", "versta russian"),
        ("arshin", "arshin", "0.7112", "russian"),
        ("sazhen", "sazhen", "2.1336", "russian"),
        ("chinese li", "li(cn)", "500", "china"),
        ("japanese shaku", "shaku", "10/33", "japan"),
        ("roman mile", "mp", "1479", "mille passus"),
        ("smoot", "smoot", "67*inch", "mit"),
        ("football field", "fbf", "100*yd", "american"),
    ]);
    b.scale("AWG wire gauge (diameter)", "AWG", Scale::Awg, "american wire gauge cable diameter electronics");

    b.category("mass", "Mass & Weight", false);
    b.si("gram", "g", "1e-3", false, &["k", "m", "µ"], "grams weight");
    b.linear(&[
        ("tonne", "t", "1000", "metric ton"),
        ("pound", "lb", "lb", "lbs pounds"),
        ("ounce", "oz", "lb/16", "ounces"),
        ("stone", "st", "14*lb", ""),
        ("short ton", "ton(US)", "2000*lb", "us ton"),
        ("long ton", "ton(UK)", "2240*lb", "imperial ton"),
        ("hundredweight (US)", "cwt(US)", "100*lb", "short"),
        ("hundredweight (UK)", "cwt(UK)", "112*lb", "long"),
        ("grain", "gr", "lb/7000", ""),
        ("dram", "dr", "lb/256", "drachm"),
        ("scruple", "℈", "20*lb/7000", "apothecary"),
        ("troy ounce", "ozt", "480*lb/7000", "gold silver"),
        ("troy pound", "lbt", "5760*lb/7000", ""),
        ("pennyweight", "dwt", "24*lb/7000", "troy"),
        ("carat", "ct", "2e-4", "gem diamond"),
        ("quintal", "q", "100", "centner"),
        ("slug", "slug", "lbf/ft", "imperial"),
        ("kip", "kip", "1000*lb", "kilopound"),
        ("pood", "pood", "16.3804964", "russian"),
        ("chinese jin", "jin", "0.5", "catty china"),
        ("tael", "tael", "0.05", "china liang"),
        ("dalton", "Da", "1.66053906660e-27", "atomic mass unit amu u"),
        ("electron mass", "mₑ", "9.1093837015e-31", ""),
        ("proton mass", "mₚ", "1.67262192369e-27", ""),
        ("planck mass", "mP", "2.176434e-8", ""),
        ("earth mass", "M⊕", "5.9722e24", ""),
        ("solar mass", "M☉", "1.98847e30", "sun"),
        ("stick of butter (US)", "stick(lb)", "lb/4", "cooking baking"),
    ]);

    b.category("volume", "Volume", false);
    b.si("liter", "L", "1e-3", false, &["m", "c", "d", "h", "k"], "litre l");
    b.linear(&[
        ("cubic meter", "m³", "1", "m3 cubic metre"),
        ("cubic kilometer", "km³", "1e9", "km3"),
        ("cubic centimeter", "cm³", "1e-6", "cm3 cc"),
        ("cubic millimeter", "mm³", "1e-9", "mm3"),
        ("cubic inch", "in³", "inch^3", "in3 cu in"),
        ("cubic foot", "ft³", "ft^3", "ft3 cu ft"),
        ("cubic yard", "yd³", "yd^3", "yd3"),
        ("cubic mile", "mi³", "mi^3", "mi3"),
        ("acre-foot", "ac⋅ft", "43560*ft^3", "acre foot"),
        ("US gallon", "gal", "gal", "gallon us liquid"),
        ("US quart", "qt", "gal/4", "quart"),
        ("US pint", "pt(US)", "gal/8", "pint"),
        ("US cup", "cup", "gal/16", "cooking"),
        ("US gill", "gi", "gal/32", ""),
        ("US fluid ounce", "fl oz", "gal/128", "floz ounce"),
        ("US tablespoon", "tbsp", "gal/256", "tablespoon cooking"),
        ("US teaspoon", "tsp", "gal/768", "teaspoon cooking"),
        ("US fluid dram", "fl dr", "gal/1024", ""),
        ("minim", "min", "gal/61440", ""),
        ("imperial gallon", "gal(UK)", "galuk", "uk gallon"),
        ("imperial quart", "qt(UK)", "galuk/4", "uk"),
        ("imperial pint", "pt(UK)", "galuk/8", "uk pint beer"),
        ("imperial cup", "cup(UK)", "galuk/16", "uk"),
        ("imperial fluid ounce", "fl oz(UK)", "galuk/160", "uk"),
        ("metric cup", "cup(m)", "2.5e-4", "cooking"),
        ("metric tablespoon", "tbsp(m)", "1.5e-5", "cooking"),
        ("metric teaspoon", "tsp(m)", "5e-6", "cooking"),
        ("drop", "gtt", "5e-8", "medical"),
        ("shot", "shot", "1.5*gal/128", "jigger"),
        ("US dry gallon", "gal(dry)", "268.8025*inch^3", "dry"),
        ("US bushel", "bu", "2150.42*inch^3", "dry"),
        ("US peck", "pk", "2150.42*inch^3/4", "dry"),
        ("oil barrel", "bbl", "42*gal", "petroleum oil"),
        ("beer barrel (US)", "bbl(beer)", "31*gal", ""),
        ("hogshead", "hhd", "63*gal", "wine"),
        ("cord", "cd", "128*ft^3", "firewood"),
        ("board foot", "FBM", "144*inch^3", "lumber timber"),
        ("stere", "st(stere)", "1", "firewood"),
        ("stick of butter (US, volume)", "stick", "gal/32", "cooking baking half cup"),
        ("wine bottle", "btl", "7.5e-4", "750 ml standard bottle"),
        ("can (12 US fl oz)", "can", "12*gal/128", "soda beer"),
        ("dessert spoon", "dsp", "1e-5", "cooking"),
    ]);

    b.category("area", "Area", false);
    b.linear(&[
        ("square meter", "m²", "1", "m2 sq metre"),
        ("square kilometer", "km²", "1e6", "km2 sq"),
        ("square centimeter", "cm²", "1e-4", "cm2 sq"),
        ("square millimeter", "mm²", "1e-6", "mm2 sq"),
        ("square micrometer", "µm²", "1e-12", "um2 sq"),
        ("hectare", "ha", "1e4", ""),
        ("are", "a", "100", "sotka сотка hundredth"),
        ("decare", "daa", "1000", "dunam stremma"),
        ("square inch", "in²", "inch^2", "in2 sq"),
        ("square foot", "ft²", "ft^2", "ft2 sq feet"),
        ("square yard", "yd²", "yd^2", "yd2 sq"),
        ("square mile", "mi²", "mi^2", "mi2 sq section"),
        ("acre", "ac", "43560*ft^2", "acres"),
        ("rood", "ro", "10890*ft^2", ""),
        ("square rod", "rd²", "(16.5*ft)^2", "perch pole"),
        ("township", "twp", "36*(6336/3937*1000)^2", "survey"),
        ("square nautical mile", "nmi²", "nmi^2", "sq"),
        ("circular mil", "cmil", "pi/4*(inch/1000)^2", "wire"),
        ("barn", "b", "1e-28", "nuclear cross section"),
        ("tsubo", "tsubo", "400/121", "japan"),
        ("football field (area)", "fbf²", "360*ft*160*ft", "american"),
    ]);

    b.category("time", "Time", false);
    b.si("second", "s", "1", false, &["m", "µ", "n", "p"], "sec seconds");
    b.linear(&[
        ("minute", "min", "60", "mins minutes"),
        ("hour", "h", "3600", "hr hours"),
        ("day", "d", "day", "days"),
        ("week", "wk", "7*day", "weeks"),
        ("fortnight", "fn", "14*day", ""),
        ("month (average)", "mo", "yr/12", "months"),
        ("quarter", "qtr", "yr/4", ""),
        ("year (365 days)", "yr", "365*day", "common year"),
        ("gregorian year", "a", "yr", "year"),
        ("julian year", "a(j)", "365.25*day", "astronomy"),
        ("leap year", "yr(leap)", "366*day", ""),
        ("decade", "dec", "10*yr", ""),
        ("century", "c", "100*yr", ""),
        ("millennium", "ka", "1000*yr", ""),
        ("sidereal day", "sd", "86164.0905", "astronomy"),
        ("sidereal year", "yr(sid)", "31558149.7635", "astronomy"),
        ("synodic month", "mo(lunar)", "29.530588*day", "lunar moon"),
        ("shake", "shake", "1e-8", "nuclear"),
        ("jiffy", "jiffy", "0.01", ""),
        ("moment", "moment", "90", "medieval"),
        ("planck time", "tP", "5.391247e-44", ""),
    ]);
    b.add("clock duration (h:mm:ss)", "h:mm:ss", Kind::Duration(DurFmt::Clock), "duration hms hours minutes seconds stopwatch timer", true);
    b.add("duration in words", "1h 5min", Kind::Duration(DurFmt::Words), "duration readable human", true);

    b.category("temperature", "Temperature", false);
    b.affine("Celsius", "°C", "1", "273.15", "centigrade c");
    b.affine("Fahrenheit", "°F", "5/9", "459.67", "f");
    b.affine("Kelvin", "K", "1", "0", "absolute");
    b.affine("Rankine", "°R", "5/9", "0", "");
    b.affine("Réaumur", "°Ré", "5/4", "218.52", "reaumur");
    b.affine("Delisle", "°De", "-2/3", "-559.725", "");
    b.affine("Newton (temperature)", "°N", "100/33", "90.1395", "");
    b.affine("Rømer", "°Rø", "40/21", "135.90375", "romer");
    b.affine("millikelvin", "mK", "1e-3", "0", "");
    b.affine("gas mark (oven)", "GM", "125/9", "709.67/25", "gas mark oven cooking baking uk");

    b.category("speed", "Speed", false);
    b.linear(&[
        ("meter per second", "m/s", "1", "mps"),
        ("kilometer per hour", "km/h", "1/3.6", "kph kmh"),
        ("mile per hour", "mph", "mi/3600", "mi/h"),
        ("knot", "kn", "nmi/3600", "kt nautical"),
        ("foot per second", "ft/s", "ft", "fps"),
        ("foot per minute", "ft/min", "ft/60", "fpm"),
        ("inch per second", "in/s", "inch", "ips"),
        ("centimeter per second", "cm/s", "0.01", ""),
        ("millimeter per second", "mm/s", "0.001", ""),
        ("meter per minute", "m/min", "1/60", ""),
        ("kilometer per second", "km/s", "1000", ""),
        ("mile per second", "mi/s", "mi", ""),
        ("mach (ISA sea level)", "Ma", "340.294", "sound"),
        ("speed of light", "c", "c", "light"),
        ("furlong per fortnight", "fur/fn", "660*ft/(14*day)", "silly"),
    ]);
    b.reciprocal("minute per kilometer (pace)", "min/km", "1000/60", "pace running jogging type 5:30 for 5 min 30 s");
    b.reciprocal("minute per mile (pace)", "min/mi", "mi/60", "pace running jogging");
    b.reciprocal("second per 100 m (swim pace)", "s/100m", "100", "swimming pace");
    b.scale("Beaufort (wind force)", "Bft", Scale::Beaufort, "beaufort wind scale weather");

    b.category("acceleration", "Acceleration", false);
    b.linear(&[
        ("meter per second squared", "m/s²", "1", "m/s2"),
        ("standard gravity", "g₀", "g0", "g force gee"),
        ("foot per second squared", "ft/s²", "ft", "ft/s2"),
        ("inch per second squared", "in/s²", "inch", "in/s2"),
        ("gal", "Gal", "0.01", "galileo"),
        ("kilometer per hour per second", "km/h/s", "1/3.6", ""),
        ("mile per hour per second", "mph/s", "mi/3600", ""),
    ]);

    b.category("pressure", "Pressure", false);
    b.si("pascal", "Pa", "1", false, &["h", "k", "M", "G"], "");
    b.linear(&[
        ("bar", "bar", "1e5", ""),
        ("millibar", "mbar", "100", "mb"),
        ("atmosphere", "atm", "101325", "standard"),
        ("technical atmosphere", "at", "g0*1e4", "kgf/cm2 kgf/cm²"),
        ("torr", "Torr", "101325/760", ""),
        ("millimeter of mercury", "mmHg", "mmhg", "blood"),
        ("inch of mercury", "inHg", "mmhg*25.4", ""),
        ("pound per square inch", "psi", "lbf/inch^2", "lbf/in2 tire"),
        ("kilopound per square inch", "ksi", "1000*lbf/inch^2", ""),
        ("pound per square foot", "psf", "lbf/ft^2", ""),
        ("centimeter of water", "cmH₂O", "g0*10", "cmh2o"),
        ("inch of water", "inH₂O", "g0*25.4", "inh2o"),
        ("meter of water", "mH₂O", "g0*1000", "mh2o"),
        ("barye", "Ba", "0.1", "dyn/cm2"),
        ("pieze", "pz", "1000", ""),
    ]);
    b.scale("sound pressure level", "dB SPL", Scale::DbSpl, "decibel loudness noise acoustic spl");

    b.category("energy", "Energy", false);
    b.si("joule", "J", "1", false, &["k", "M", "G", "m"], "");
    b.linear(&[
        ("calorie", "cal", "4.184", "thermochemical"),
        ("kilocalorie", "kcal", "4184", "food calorie Cal"),
        ("IT calorie", "cal(IT)", "4.1868", ""),
        ("milliwatt-hour", "mWh", "3.6", ""),
        ("watt-hour", "Wh", "3600", ""),
        ("kilowatt-hour", "kWh", "3.6e6", "electricity"),
        ("megawatt-hour", "MWh", "3.6e9", ""),
        ("gigawatt-hour", "GWh", "3.6e12", ""),
        ("terawatt-hour", "TWh", "3.6e15", ""),
        ("electronvolt", "eV", "ev", ""),
        ("millielectronvolt", "meV", "ev*1e-3", ""),
        ("kiloelectronvolt", "keV", "ev*1e3", ""),
        ("megaelectronvolt", "MeV", "ev*1e6", ""),
        ("gigaelectronvolt", "GeV", "ev*1e9", ""),
        ("teraelectronvolt", "TeV", "ev*1e12", ""),
        ("British thermal unit", "BTU", "btu", "btu"),
        ("therm", "thm", "105480400", "gas"),
        ("quad", "quad", "btu*1e15", ""),
        ("erg", "erg", "1e-7", "cgs"),
        ("foot-pound", "ft⋅lbf", "ft*lbf", "ft-lb"),
        ("inch-pound", "in⋅lbf", "inch*lbf", "in-lb"),
        ("horsepower-hour", "hp⋅h", "550*ft*lbf*3600", ""),
        ("ton of TNT", "tTNT", "4.184e9", "explosive"),
        ("kiloton of TNT", "ktTNT", "4.184e12", "explosive"),
        ("megaton of TNT", "MtTNT", "4.184e15", "explosive"),
        ("barrel of oil equivalent", "BOE", "6.1178632e9", ""),
        ("tonne of oil equivalent", "toe", "4.1868e10", ""),
        ("tonne of coal equivalent", "tce", "2.9307e10", ""),
        ("hartree", "Eₕ", "4.3597447222071e-18", "atomic"),
        ("rydberg", "Ry", "2.1798723611035e-18", "atomic"),
        ("planck energy", "EP", "1.9561e9", ""),
    ]);

    b.category("power", "Power", false);
    b.si("watt", "W", "1", false, &["k", "M", "G", "m"], "");
    b.linear(&[
        ("horsepower", "hp", "550*ft*lbf", "mechanical imperial"),
        ("metric horsepower", "PS", "75*g0", "ps cv pk"),
        ("electric horsepower", "hp(E)", "746", ""),
        ("boiler horsepower", "hp(S)", "9809.5", ""),
        ("BTU per hour", "BTU/h", "btu/3600", ""),
        ("BTU per second", "BTU/s", "btu", ""),
        ("ton of refrigeration", "TR", "12000*btu/3600", "ac cooling"),
        ("erg per second", "erg/s", "1e-7", ""),
        ("foot-pound per second", "ft⋅lbf/s", "ft*lbf", ""),
        ("calorie per second", "cal/s", "4.184", ""),
        ("kilocalorie per hour", "kcal/h", "4184/3600", ""),
        ("solar luminosity", "L☉", "3.828e26", "sun"),
    ]);
    b.scale("decibel-milliwatt", "dBm", Scale::Dbm, "dbm radio rf signal level electronics");
    b.scale("decibel-watt", "dBW", Scale::Dbw, "dbw radio rf signal level electronics");

    b.category("force", "Force", false);
    b.si("newton", "N", "1", false, &["k", "M", "m"], "");
    b.linear(&[
        ("dyne", "dyn", "1e-5", "cgs"),
        ("kilogram-force", "kgf", "g0", "kilopond kp"),
        ("gram-force", "gf", "g0/1000", ""),
        ("tonne-force", "tf", "g0*1000", ""),
        ("pound-force", "lbf", "lbf", ""),
        ("kip-force", "kipf", "lbf*1000", "kip"),
        ("ounce-force", "ozf", "lbf/16", ""),
        ("poundal", "pdl", "lb*ft", ""),
        ("sthène", "sn", "1000", "sthene"),
    ]);

    b.category("torque", "Torque", false);
    b.linear(&[
        ("newton meter", "N⋅m", "1", "nm n-m"),
        ("kilonewton meter", "kN⋅m", "1000", ""),
        ("newton centimeter", "N⋅cm", "0.01", ""),
        ("newton millimeter", "N⋅mm", "0.001", ""),
        ("kilogram-force meter", "kgf⋅m", "g0", ""),
        ("kilogram-force centimeter", "kgf⋅cm", "g0/100", ""),
        ("pound-force foot", "lbf⋅ft", "lbf*ft", "ft-lb ft lbs"),
        ("pound-force inch", "lbf⋅in", "lbf*inch", "in-lb"),
        ("ounce-force inch", "ozf⋅in", "lbf/16*inch", ""),
        ("dyne centimeter", "dyn⋅cm", "1e-7", ""),
    ]);

    b.category("density", "Density", false);
    b.linear(&[
        ("kilogram per cubic meter", "kg/m³", "1", "kg/m3"),
        ("gram per cubic centimeter", "g/cm³", "1000", "g/cm3 g/cc"),
        ("gram per milliliter", "g/mL", "1000", "g/ml"),
        ("kilogram per liter", "kg/L", "1000", "kg/l"),
        ("gram per liter", "g/L", "1", "g/l"),
        ("milligram per liter", "mg/L", "1e-3", "mg/l"),
        ("tonne per cubic meter", "t/m³", "1000", "t/m3"),
        ("pound per cubic foot", "lb/ft³", "lb/ft^3", "lb/ft3"),
        ("pound per cubic inch", "lb/in³", "lb/inch^3", "lb/in3"),
        ("pound per US gallon", "lb/gal", "lb/gal", ""),
        ("pound per imperial gallon", "lb/gal(UK)", "lb/galuk", ""),
        ("ounce per cubic inch", "oz/in³", "lb/16/inch^3", "oz/in3"),
        ("slug per cubic foot", "slug/ft³", "lbf/ft/ft^3", ""),
    ]);

    b.category("fuel", "Fuel Economy", false);
    b.linear(&[
        ("kilometer per liter", "km/L", "1", "kmpl"),
        ("mile per gallon (US)", "mpg", "mi/1000/(gal*1000)", "mpg us"),
        ("mile per gallon (UK)", "mpg(UK)", "mi/1000/(galuk*1000)", "imperial"),
        ("mile per liter", "mi/L", "mi/1000", ""),
    ]);
    b.reciprocal("liter per 100 km", "L/100km", "100", "l/100km consumption");
    b.reciprocal("liter per kilometer", "L/km", "1", "");
    b.reciprocal("US gallon per 100 miles", "gal/100mi", "100*mi/1000/(gal*1000)", "");

    b.category("flow", "Flow Rate", false);
    b.linear(&[
        ("cubic meter per second", "m³/s", "1", "m3/s cumec"),
        ("cubic meter per hour", "m³/h", "1/3600", "m3/h"),
        ("liter per second", "L/s", "1e-3", "l/s"),
        ("liter per minute", "L/min", "1e-3/60", "lpm"),
        ("liter per hour", "L/h", "1e-3/3600", "l/h"),
        ("milliliter per minute", "mL/min", "1e-6/60", ""),
        ("US gallon per minute", "gpm", "gal/60", "gal/min"),
        ("US gallon per hour", "gph", "gal/3600", "gal/h"),
        ("imperial gallon per minute", "gpm(UK)", "galuk/60", ""),
        ("cubic foot per second", "cfs", "ft^3", "ft3/s"),
        ("cubic foot per minute", "cfm", "ft^3/60", "ft3/min"),
        ("barrel per day", "bbl/d", "42*gal/day", "oil bpd"),
    ]);

    b.category("data", "Data Storage", false);
    b.si("bit", "b", "1", true, &["k", "M", "G"], "bits");
    b.si("byte", "B", "8", true, &["k", "M", "G", "T", "P"], "bytes octet");
    b.linear(&[("nibble", "nibble", "4", "half byte")]);
    for (i, (pn, ps)) in [("kibi", "Ki"), ("mebi", "Mi"), ("gibi", "Gi"), ("tebi", "Ti"), ("pebi", "Pi"), ("exbi", "Ei"), ("zebi", "Zi"), ("yobi", "Yi")]
        .into_iter()
        .enumerate()
    {
        let p = format!("2^{}", 10 * (i + 1));
        let kind = super::Kind::Linear;
        b.unit(&format!("{pn}byte"), &format!("{ps}B"), kind, super::expr(&format!("8*{p}")), crate::num::int(0), "binary iec", i < 5);
        b.unit(&format!("{pn}bit"), &format!("{ps}bit"), kind, super::expr(&p), crate::num::int(0), "binary iec", i < 3);
    }

    b.category("datarate", "Data Rate", false);
    b.linear(&[
        ("bit per second", "bit/s", "1", "bps"),
        ("kilobit per second", "kbit/s", "1e3", "kbps"),
        ("megabit per second", "Mbit/s", "1e6", "mbps internet"),
        ("gigabit per second", "Gbit/s", "1e9", "gbps"),
        ("terabit per second", "Tbit/s", "1e12", "tbps"),
        ("byte per second", "B/s", "8", ""),
        ("kilobyte per second", "kB/s", "8e3", "kbs"),
        ("megabyte per second", "MB/s", "8e6", "download"),
        ("gigabyte per second", "GB/s", "8e9", ""),
        ("terabyte per second", "TB/s", "8e12", ""),
        ("kibibyte per second", "KiB/s", "8*2^10", ""),
        ("mebibyte per second", "MiB/s", "8*2^20", ""),
        ("gibibyte per second", "GiB/s", "8*2^30", ""),
    ]);

    b.category("angle", "Angle", false);
    b.linear(&[
        ("degree", "°", "pi/180", "deg degrees"),
        ("radian", "rad", "1", "radians"),
        ("milliradian", "mrad", "1e-3", ""),
        ("gradian", "grad", "pi/200", "gon grade"),
        ("arcminute", "′", "pi/10800", "arcmin minute of arc"),
        ("arcsecond", "″", "pi/648000", "arcsec second of arc"),
        ("milliarcsecond", "mas", "pi/648000000", ""),
        ("turn", "tr", "2*pi", "revolution rev cycle"),
        ("quadrant", "quad", "pi/2", "right angle"),
        ("sextant", "sextant", "pi/3", ""),
        ("NATO mil", "mil(NATO)", "2*pi/6400", "military"),
        ("hour angle", "HA", "pi/12", "astronomy"),
        ("compass point", "point", "2*pi/32", ""),
        ("zodiac sign", "sign", "pi/6", ""),
    ]);

    b.category("frequency", "Frequency", false);
    b.si("hertz", "Hz", "1", false, &["k", "M", "G", "T"], "cycles per second");
    b.linear(&[
        ("revolution per minute", "rpm", "1/60", "r/min"),
        ("beats per minute", "bpm", "1/60", "heart tempo"),
        ("radian per second", "rad/s", "1/(2*pi)", "angular"),
        ("degree per second", "°/s", "1/360", ""),
        ("cycle per hour", "cph", "1/3600", ""),
    ]);

    b.category("ratio", "Ratio & Percent", false);
    b.linear(&[
        ("percent", "%", "1/100", "percentage pct"),
        ("decimal fraction", "×1", "1", "ratio factor"),
        ("per mille", "‰", "1/1000", "permille promille"),
        ("basis point", "bp", "1/10000", "bps permyriad ‱ finance"),
        ("parts per million", "ppm", "1e-6", ""),
        ("parts per billion", "ppb", "1e-9", ""),
        ("parts per trillion", "ppt", "1e-12", ""),
        ("parts per quadrillion", "ppq", "1e-15", ""),
    ]);
    b.add("fraction", "a/b", Kind::Fraction(Frac::Fraction), "fraction rational numerator denominator exact math", true);
    b.add("mixed number", "a b/c", Kind::Fraction(Frac::Mixed), "mixed fraction whole math", true);
    b.reciprocal("one in N (odds)", "1:N", "1", "probability chance odds one in");
    b.scale("decibel (power ratio)", "dB", Scale::DbPower, "decibel gain loss power ratio electronics audio");
    b.scale("decibel (amplitude ratio)", "dB(V)", Scale::DbAmplitude, "decibel voltage field amplitude gain electronics audio");

    b.category("datetime", "Date & Time", false);
    b.add("ISO 8601 date (UTC)", "ISO", Kind::Date(DateFmt::Iso), "date time datetime iso rfc3339 utc calendar", true);
    b.linear(&[
        ("Unix timestamp (seconds)", "unix", "1", "epoch posix time timestamp seconds"),
        ("Unix timestamp (milliseconds)", "unix ms", "1e-3", "epoch javascript java time timestamp millis"),
        ("Unix timestamp (microseconds)", "unix µs", "1e-6", "epoch timestamp micros"),
        ("Unix timestamp (nanoseconds)", "unix ns", "1e-9", "epoch timestamp nanos go"),
    ]);
    b.add("RFC 2822 date", "RFC", Kind::Date(DateFmt::Rfc2822), "email http header date", true);
    b.add("Readable date", "date", Kind::Date(DateFmt::Readable), "weekday day of week human long date", true);
    b.add("ISO week date", "YYYY-Www-D", Kind::Date(DateFmt::IsoWeek), "week number calendar week kw", true);
    b.add("Ordinal date (day of year)", "YYYY-DDD", Kind::Date(DateFmt::Ordinal), "day of year doy julian ordinal", true);
    for (name, sym, offset, kw) in [
        ("Julian Day", "JD", "-2440587.5", "astronomy jd"),
        ("Modified Julian Day", "MJD", "-40587", "astronomy mjd"),
        ("Excel serial date (1900)", "Excel", "-25569", "spreadsheet lotus sheets date number oadate ole automation vba"),
    ] {
        b.unit(name, sym, Kind::Affine, super::expr("86400"), super::expr(offset), kw, true);
    }
    b.unit("Windows FILETIME (100 ns)", "FILETIME", Kind::Affine, super::expr("1e-7"), super::expr("-116444736000000000"), "windows ntfs filetime 1601 epoch", true);
    b.unit("Mac HFS+ timestamp", "HFS+", Kind::Affine, super::expr("1"), super::expr("-2082844800"), "apple classic mac epoch 1904", true);
    b.unit("GPS time (seconds)", "GPS", Kind::Affine, super::expr("1"), super::expr("315964800-18"), "gps epoch 1980 navigation (current 18 s leap offset)", true);

    b.category("spectrum", "Waves & Photons", false);
    b.linear(&[
        ("frequency (Hz)", "Hz(ν)", "1", "frequency hertz light wave"),
        ("frequency (kHz)", "kHz(ν)", "1e3", ""),
        ("frequency (MHz)", "MHz(ν)", "1e6", "radio"),
        ("frequency (GHz)", "GHz(ν)", "1e9", "radio microwave wifi"),
        ("frequency (THz)", "THz(ν)", "1e12", "infrared light"),
        ("frequency (PHz)", "PHz(ν)", "1e15", "ultraviolet light"),
        ("photon energy (eV)", "eV(γ)", "ev/planck", "photon energy electronvolt quantum"),
        ("photon energy (meV)", "meV(γ)", "ev/planck/1000", ""),
        ("photon energy (keV)", "keV(γ)", "ev/planck*1000", "x-ray"),
        ("gamma photon energy (MeV)", "MeV(γ)", "ev/planck*1e6", "gamma photon energy"),
        ("photon energy (J)", "J(γ)", "1/planck", "joule"),
        ("wavenumber (cm⁻¹)", "cm⁻¹", "c*100", "wavenumber spectroscopy kayser infrared"),
        ("wavenumber (m⁻¹)", "m⁻¹", "c", "wavenumber"),
    ]);
    for (name, sym, factor, kw) in [
        ("wavelength (m)", "λ m", "c", "wavelength light wave"),
        ("wavelength (cm)", "λ cm", "c*100", "wavelength microwave"),
        ("wavelength (mm)", "λ mm", "c*1000", "wavelength millimeter wave"),
        ("wavelength (µm)", "λ µm", "c*1e6", "wavelength infrared micron um"),
        ("wavelength (nm)", "λ nm", "c*1e9", "wavelength light color visible laser nanometer"),
        ("wavelength (pm)", "λ pm", "c*1e12", "wavelength x-ray"),
        ("wavelength (Å)", "λ Å", "c*1e10", "wavelength angstrom"),
        ("period (s)", "T s", "1", "period oscillation cycle time"),
        ("period (ms)", "T ms", "1e3", "period"),
        ("period (µs)", "T µs", "1e6", "period"),
        ("period (ns)", "T ns", "1e9", "period"),
    ] {
        b.reciprocal(name, sym, factor, kw);
    }

    b.category("molarity", "Molar Concentration", false);
    b.linear(&[
        ("molar (mol/L)", "M", "1", "molarity mol/l concentration chemistry"),
        ("millimolar", "mM", "1e-3", "mmol/l"),
        ("micromolar", "µM", "1e-6", "umol/l um"),
        ("nanomolar", "nM", "1e-9", "nmol/l"),
        ("picomolar", "pM", "1e-12", "pmol/l"),
        ("mole per cubic meter", "mol/m³", "1e-3", "mol/m3"),
        ("mole per milliliter", "mol/mL", "1e3", ""),
    ]);
    b.scale("pH (from H⁺ concentration)", "pH", Scale::Ph, "ph acidity hydrogen ion chemistry");

    b.category("glucose", "Blood Glucose", false);
    b.linear(&[
        ("millimole per liter (glucose)", "mmol/L", "1", "blood sugar glucose diabetes mmol"),
        ("milligram per deciliter (glucose)", "mg/dL", "10/180.156", "blood sugar glucose diabetes mg/dl"),
        ("gram per liter (glucose)", "g/L(glu)", "1000/180.156", "blood sugar glucose"),
    ]);
    b.category("hba1c", "HbA1c", false);
    b.linear(&[("IFCC (mmol/mol)", "mmol/mol", "1", "hba1c a1c ifcc diabetes")]);
    b.unit("NGSP / DCCT (%)", "%A1c", Kind::Affine, super::expr("10.929"), super::expr("-2.15"), "hba1c a1c dcct ngsp diabetes percent", true);

    b.category("solidangle", "Solid Angle", false);
    b.linear(&[
        ("steradian", "sr", "1", "solid angle"),
        ("millisteradian", "msr", "1e-3", ""),
        ("square degree", "deg²", "(pi/180)^2", "sq deg sky area astronomy"),
        ("square arcminute", "arcmin²", "(pi/10800)^2", "sky astronomy"),
        ("square arcsecond", "arcsec²", "(pi/648000)^2", "sky astronomy"),
        ("spat (full sphere)", "sp", "4*pi", "sphere"),
        ("hemisphere", "hemi", "2*pi", ""),
    ]);

    b.category("viscosity", "Dynamic Viscosity", false);
    b.linear(&[
        ("pascal second", "Pa⋅s", "1", "viscosity pa s"),
        ("millipascal second", "mPa⋅s", "1e-3", "viscosity water"),
        ("poise", "P", "0.1", "viscosity cgs"),
        ("centipoise", "cP", "1e-3", "viscosity cps"),
        ("pound per foot second", "lb/(ft⋅s)", "lb/ft", "viscosity"),
        ("reyn", "reyn", "lbf/inch^2", "viscosity"),
    ]);
    b.category("kinvisc", "Kinematic Viscosity", false);
    b.linear(&[
        ("square meter per second", "m²/s", "1", "kinematic viscosity"),
        ("square millimeter per second", "mm²/s", "1e-6", "kinematic viscosity oil"),
        ("stokes", "St", "1e-4", "kinematic viscosity"),
        ("centistokes", "cSt", "1e-6", "kinematic viscosity oil"),
        ("square foot per second", "ft²/s", "ft^2", "kinematic viscosity"),
    ]);

    b.category("specenergy", "Specific Energy", false);
    b.linear(&[
        ("joule per kilogram", "J/kg", "1", "specific energy heating value"),
        ("kilojoule per kilogram", "kJ/kg", "1e3", "latent heat"),
        ("megajoule per kilogram", "MJ/kg", "1e6", "fuel energy density"),
        ("watt-hour per kilogram", "Wh/kg", "3600", "battery energy density"),
        ("kilowatt-hour per kilogram", "kWh/kg", "3.6e6", "fuel hydrogen"),
        ("BTU per pound", "BTU/lb", "btu/lb", "fuel"),
        ("calorie per gram", "cal/g", "4184", ""),
        ("kilocalorie per 100 g (food)", "kcal/100g", "41840", "food nutrition label"),
        ("kilojoule per 100 g (food)", "kJ/100g", "1e4", "food nutrition label"),
    ]);

    b.category("css", "CSS & Screen (96 dpi)", false);
    b.linear(&[
        ("CSS pixel", "px(css)", "1", "css web pixel screen"),
        ("CSS point", "pt(css)", "4/3", "css font size"),
        ("CSS pica", "pc(css)", "16", "css"),
        ("CSS inch", "in(css)", "96", "css"),
        ("CSS centimeter", "cm(css)", "96/2.54", "css"),
        ("CSS millimeter", "mm(css)", "96/25.4", "css"),
        ("CSS quarter-millimeter", "Q", "96/101.6", "css"),
        ("rem / em (16 px root)", "rem", "16", "css font size em root default"),
        ("Android dp (mdpi)", "dp", "1", "android density independent pixel"),
    ]);

    b.category("shoe", "Shoe Size (adult)", false);
    b.linear(&[
        ("shoe last length (cm)", "last cm", "1", "shoe length foot"),
        ("shoe last length (inch)", "last in", "2.54", "shoe length"),
    ]);
    for (name, sym, factor, offset, kw) in [
        ("EU shoe size (Paris point)", "EU", "2/3", "0", "europe european shoe size"),
        ("UK shoe size", "UK", "2.54/3", "25", "british shoe size"),
        ("US men's shoe size", "US M", "2.54/3", "24", "american shoe size men"),
        ("US women's shoe size", "US W", "2.54/3", "22.5", "american shoe size women"),
    ] {
        b.unit(name, sym, Kind::Affine, super::expr(factor), super::expr(offset), kw, true);
    }

    // Electromagnetism and other SI quantities: base unit + prefixes.
    let si_cats: &[(&str, &str, &str, &str, &[&str], &str)] = &[
        ("voltage", "Voltage", "volt", "V", &["k", "m", "µ", "M"], "potential emf"),
        ("current", "Electric Current", "ampere", "A", &["k", "m", "µ", "n"], "amp amps"),
        ("resistance", "Resistance", "ohm", "Ω", &["k", "M", "m", "G"], "ohms"),
        ("conductance", "Conductance", "siemens", "S", &["m", "µ", "k"], "mho"),
        ("charge", "Electric Charge", "coulomb", "C", &["m", "µ", "n", "p"], ""),
        ("capacitance", "Capacitance", "farad", "F", &["m", "µ", "n", "p"], "capacitor"),
        ("inductance", "Inductance", "henry", "H", &["m", "µ", "n"], "coil"),
        ("magfield", "Magnetic Flux Density", "tesla", "T", &["m", "µ", "n"], "magnetic field"),
        ("magflux", "Magnetic Flux", "weber", "Wb", &["m", "µ"], ""),
        ("substance", "Amount of Substance", "mole", "mol", &["k", "m", "µ", "n"], "chemistry"),
        ("radioactivity", "Radioactivity", "becquerel", "Bq", &["k", "M", "G"], "decay"),
        ("absorbed", "Absorbed Dose", "gray", "Gy", &["m", "µ"], "radiation"),
        ("equivalent", "Equivalent Dose", "sievert", "Sv", &["m", "µ", "n"], "radiation"),
        ("illuminance", "Illuminance", "lux", "lx", &["k", "m"], "light"),
        ("luminous", "Luminous Intensity", "candela", "cd", &["k", "m"], "candle"),
        ("lumflux", "Luminous Flux", "lumen", "lm", &["k", "m"], "light bulb"),
        ("catalytic", "Catalytic Activity", "katal", "kat", &["m", "µ", "n"], "enzyme"),
    ];
    for &(id, name, unit, sym, common, kw) in si_cats {
        b.category(id, name, false);
        b.si(unit, sym, "1", false, common, kw);
        let extras: &[(&str, &str, &str, &str)] = match id {
            "current" => &[("abampere", "abA", "10", "biot cgs"), ("statampere", "statA", "1/(10*c)", "cgs")],
            "charge" => &[
                ("ampere-hour", "Ah", "3600", "battery"),
                ("milliampere-hour", "mAh", "3.6", "battery"),
                ("elementary charge", "e", "ev", "electron"),
                ("faraday", "Fd", "ev*na", ""),
                ("statcoulomb", "statC", "1/(10*c)", "esu franklin"),
            ],
            "magfield" => &[("gauss", "Gs", "1e-4", "G cgs"), ("gamma", "γ", "1e-9", "")],
            "magflux" => &[("maxwell", "Mx", "1e-8", "cgs")],
            "substance" => &[("number of particles", "N", "1/na", "entities molecules atoms avogadro")],
            "radioactivity" => &[
                ("curie", "Ci", "3.7e10", ""),
                ("millicurie", "mCi", "3.7e7", ""),
                ("microcurie", "µCi", "3.7e4", "uci"),
                ("rutherford", "Rd", "1e6", ""),
            ],
            "absorbed" => &[("rad (dose)", "rad(dose)", "0.01", "")],
            "equivalent" => &[("rem", "rem", "0.01", "roentgen equivalent man"), ("millirem", "mrem", "1e-5", "")],
            "illuminance" => &[("foot-candle", "fc", "1/ft^2", "footcandle"), ("phot", "ph", "1e4", ""), ("nox", "nx", "1e-3", "")],
            "luminous" => &[("candlepower", "cp", "0.981", ""), ("hefnerkerze", "HK", "0.903", "")],
            "catalytic" => &[("enzyme unit", "U", "1e-6/60", "")],
            _ => &[],
        };
        b.linear(extras);
    }

    b.category("stops", "Light & Exposure Stops", false);
    b.scale("stops (EV difference)", "stops", super::scale::Scale::Stops, "exposure ev f-stop photography nd filter");
    b.linear(&[("light factor (ND number)", "ND×", "1", "nd filter nd8 nd64 nd1000 neutral density photography")]);
    b.scale("optical density", "OD", super::scale::Scale::OpticalDensity, "nd 0.9 filter density photography");
    b.reciprocal("light transmission", "% light", "100", "transmittance percent filter photography");
}
