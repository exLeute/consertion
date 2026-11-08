//! Command line: one-shot conversions, search, files, translation, your own units and shell
//! completion. `run` returns None when the interactive app should start instead.

use crate::catalog::custom::{self, CustomUnit};
use crate::catalog::{self, Catalog, Kind};
use crate::convert::{self, Output};
use crate::settings::Settings;
use crate::{files, rates, translate};
use unicode_width::UnicodeWidthStr;

const USAGE: &str = "consertion — a converter for units, currencies, time zones, sizes, files and more

Usage:
  consertion                        open the app
  consertion 5 mi km                convert (also: 5 mi to km, 5mi km)
  consertion 5 mi                   show 5 mi in the common units of its kind
  consertion 15:00 kyiv to new york words are fine after \"to\"
  consertion find <words>           search units, e.g. consertion find pressure
  consertion file <in> <out>        convert a file by extension, e.g. photo.heic photo.jpg
  consertion translate <from> <to> <text>
                                    e.g. consertion translate auto de \"good morning\"
  consertion add <name> = <amount> <unit>
                                    your own unit, e.g. consertion add step = 0.75 m
  consertion add <name>             start a new kind of unit (then: add slice = 1/8 pizza)
  consertion mine                   list your units
  consertion remove <name>          delete one of your units
  consertion completions <shell>    tab completion for bash, zsh, fish or powershell
  consertion -h | -V                help | version

Units match by symbol or name (km, kilometer, \"new york\", \"cup of flour\").
Currencies and translation use the internet; nothing else does.";

/// Exit status of a finished command, or None to open the app.
pub fn run(args: &[String]) -> Option<i32> {
    let first = args.first()?.as_str();
    let result = match first {
        "-V" | "--version" => Ok(format!("consertion {}", env!("CARGO_PKG_VERSION"))),
        "-h" | "--help" | "help" => Ok(USAGE.to_string()),
        "find" | "search" => find(&args[1..]),
        "file" if args.len() == 3 => file(&args[1], &args[2]),
        "translate" if args.len() >= 4 => translate(&args[1], &args[2], &args[3..].join(" ")),
        "add" if args.len() >= 2 => add(&args[1..]),
        "remove" | "rm" if args.len() >= 2 => remove(&args[1..].join(" ")),
        "mine" => Ok(mine()),
        "completions" if args.len() == 2 => completions(&args[1]),
        "__complete" => {
            complete(&args[1..]);
            return Some(0);
        }
        "file" | "translate" | "add" | "remove" | "rm" | "completions" => Err(format!("missing arguments — see consertion --help\n\n{USAGE}")),
        _ => convert(args),
    };
    Some(match result {
        Ok(s) => {
            say(&s);
            0
        }
        Err(e) => {
            eprintln!("consertion: {e}");
            1
        }
    })
}

/// Print a line; a closed pipe (`consertion find m | head`) isn't an error.
fn say(s: &str) {
    use std::io::Write;
    let _ = writeln!(std::io::stdout().lock(), "{s}");
}

fn catalog() -> Catalog {
    Catalog::with_custom(&Settings::load().custom)
}

/// `5 mi km`, `5 mi to km`, `5mi km`, `15:00 kyiv to new york`, `5 mi`.
fn convert(args: &[String]) -> Result<String, String> {
    let c = catalog();
    let sep = args.iter().skip(2).position(|a| matches!(a.as_str(), "to" | "in" | "->" | "=")).map(|i| i + 2);
    let (value, from, to) = match (sep, args.len()) {
        (Some(i), _) => (args[0].clone(), args[1..i].join(" "), Some(args[i + 1..].join(" "))),
        (None, 1) => match glued(&c, &args[0]) {
            Some((v, u)) => (v, u, None),
            None => return Err(format!("what is \"{}\" in? e.g. consertion 5 mi km — see --help", args[0])),
        },
        (None, 2) => match glued(&c, &args[0]) {
            Some((v, u)) => (v, u, Some(args[1].clone())),
            None => (args[0].clone(), args[1].clone(), None),
        },
        (None, 3) => (args[0].clone(), args[1].clone(), Some(args[2].clone())),
        _ => return Err("too many words — quote names with spaces, or put \"to\" before the target".into()),
    };
    let value = engineering(&value).unwrap_or(value);
    if to.as_deref().is_some_and(|t| t.trim().is_empty()) {
        return Err("nothing after \"to\"".into());
    }
    let s = Settings::load();
    let usable = |u: &catalog::Unit| u.kind.can_parse() && u.kind.accepts(&value) && !matches!(u.kind, Kind::Calc(_) | Kind::File(_) | Kind::Lang(_) | Kind::NewCustom);
    let fuzzy = c.search(&from, None, usable);
    // Units named exactly as typed win over fuzzy matches ("mi" is the mile, not milligram).
    let mut exact: Vec<usize> = fuzzy.iter().copied().filter(|&u| is_named(&c.units[u], &from)).collect();
    // Exact symbol first ("C" coulomb before "c" speed of light); "F"/"C" alone mean temperatures.
    exact.sort_by_key(|&u| {
        let u = &c.units[u];
        let temp = u.symbol.starts_with('°') && from.trim().len() == 1;
        (!temp, u.symbol != from.trim())
    });
    let candidates = if exact.is_empty() { fuzzy } else { exact.clone() };
    if candidates.is_empty() {
        return Err(unknown(&c, &from, &value));
    }
    let Some(to) = to else { return table(&c, &s, candidates[0], &value) };
    // Prefer a source whose category has an exact match for the target ("awg" + "mm" → wire gauge).
    let pair = candidates
        .iter()
        .find_map(|&f| Some((f, c.exact(&to, c.units[f].category)?)))
        .or_else(|| candidates.iter().find_map(|&f| Some((f, c.lookup(&to, Some(c.units[f].category))?))))
        .filter(|&(f, t)| c.pairs(f, t));
    let Some((f, t)) = pair else {
        if exact.is_empty() {
            return Err(unknown(&c, &from, &value));
        }
        let fu = &c.units[candidates[0]];
        return Err(match c.lookup(&to, None) {
            Some(t) => format!(
                "{} is {} and {} is {} — they don't convert",
                fu.name,
                c.categories[fu.category].name.to_lowercase(),
                c.units[t].name,
                c.categories[c.units[t].category].name.to_lowercase()
            ),
            None => format!("no unit called \"{to}\" — try consertion find {to}"),
        });
    };
    one(&c, &s, f, t, &value)
}

fn is_named(u: &catalog::Unit, s: &str) -> bool {
    let s = s.trim();
    u.symbol.eq_ignore_ascii_case(s)
        || u.symbol.strip_prefix('°').is_some_and(|x| x.eq_ignore_ascii_case(s))
        || u.name.eq_ignore_ascii_case(s)
        || u.name.eq_ignore_ascii_case(&format!("{s} time"))
}

/// Values written the electronics way: "4.7k" → "4.7e3", "100n" → "100e-9", "4k7" → "4.7e3".
/// Only used when the unit is a separate word, so "5m" alone still means 5 meters.
fn engineering(v: &str) -> Option<String> {
    let i = v.find(|c: char| !(c.is_ascii_digit() || ".-+".contains(c)))?;
    let (num, rest) = v.split_at(i);
    let mut chars = rest.chars();
    let exp = match chars.next()? {
        'G' => 9,
        'M' => 6,
        'k' | 'K' => 3,
        'm' => -3,
        'u' | 'µ' => -6,
        'n' => -9,
        'p' => -12,
        'R' | 'r' => 0,
        _ => return None,
    };
    let tail = chars.as_str();
    let ok = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let num = match tail {
        "" if num.chars().any(|c| c.is_ascii_digit()) && exp != 0 => num.to_string(),
        t if ok(num) && ok(t) => format!("{num}.{t}"),
        _ => return None,
    };
    Some(format!("{num}e{exp}"))
}

/// "5mi" → ("5", "mi") when the tail is a unit.
fn glued(c: &Catalog, s: &str) -> Option<(String, String)> {
    let i = s.find(|ch: char| !(ch.is_ascii_digit() || ".,-+e".contains(ch)))?;
    let (v, u) = s.split_at(i);
    let numeric = v.chars().any(|ch| ch.is_ascii_digit()) && !v.ends_with('e');
    (numeric && !u.trim().is_empty() && !c.search(u, None, |x| x.kind.is_numeric()).is_empty()).then(|| (v.to_string(), u.trim().to_string()))
}

fn rates_for(c: &Catalog, u: usize) -> Result<Option<rates::Rates>, String> {
    if c.categories[c.units[u].category].online {
        rates::fetch().or_else(|e| rates::Rates::load_cache().ok_or(e)).map(Some)
    } else {
        Ok(None)
    }
}

fn show(s: &Settings, kind: Kind, out: Output) -> Option<String> {
    match out {
        Output::Number(v) => Some(convert::format_value(kind, &v, s.precision, s.notation).0),
        Output::Text(t) => Some(t),
        Output::Empty => None,
    }
}

fn one(c: &Catalog, s: &Settings, f: usize, t: usize, value: &str) -> Result<String, String> {
    let rates = rates_for(c, f)?;
    let out = convert::convert(&c.units[f], &c.units[t], value, rates.as_ref())?;
    show(s, c.units[t].kind, out).ok_or("the value isn't finished, e.g. 1.5 or 2026-11-07".into())
}

/// `consertion 5 mi`: the value in the first dozen units of its category.
fn table(c: &Catalog, s: &Settings, f: usize, value: &str) -> Result<String, String> {
    let rates = rates_for(c, f)?;
    let fu = &c.units[f];
    let mut rows = Vec::new();
    let targets: Vec<usize> = c.search("", Some(fu.category), |u| u.kind.can_target()).into_iter().filter(|&t| t != f && c.pairs(f, t)).collect();
    let common: Vec<usize> = targets.iter().copied().filter(|&t| c.units[t].common).collect();
    let pick = if common.len() >= 4 { common } else { targets };
    for t in pick.into_iter().take(14) {
        let tu = &c.units[t];
        if let Ok(out) = convert::convert(fu, tu, value, rates.as_ref())
            && let Some(v) = show(s, tu.kind, out)
        {
            rows.push((v, tu.symbol.clone(), tu.name.clone()));
        }
    }
    if rows.is_empty() {
        return Err(format!("can't read \"{value}\" as {}", fu.name));
    }
    let w = rows.iter().map(|r| r.0.width()).max().unwrap_or(0);
    let mut out = format!("{value} {} ({}) =\n", fu.symbol, fu.name);
    for (v, sym, name) in rows {
        let label = if name.to_lowercase().contains(&sym.to_lowercase()) { name } else { format!("{sym}  {name}") };
        out += &format!("  {}{v}  {label}\n", " ".repeat(w - v.width()));
    }
    Ok(out.trim_end().to_string())
}

fn unknown(c: &Catalog, from: &str, value: &str) -> String {
    let near: Vec<String> = c.search(from, None, |_| true).into_iter().take(4).map(|u| format!("{} ({})", c.units[u].symbol, c.units[u].name)).collect();
    if near.is_empty() {
        format!("no unit called \"{from}\" — try consertion find <words>")
    } else {
        format!("\"{value}\" isn't a value for \"{from}\". Close matches: {}", near.join(", "))
    }
}

fn find(words: &[String]) -> Result<String, String> {
    let c = catalog();
    let q = words.join(" ");
    let hits = c.search(&q, None, |u| !matches!(u.kind, Kind::NewCustom));
    if hits.is_empty() {
        return Err(format!("nothing matches \"{q}\""));
    }
    let shown: Vec<_> = hits.iter().take(30).map(|&u| (&c.units[u].symbol, &c.units[u].name, c.categories[c.units[u].category].name)).collect();
    let w = shown.iter().map(|r| r.0.width()).max().unwrap_or(0).min(18);
    let mut out: Vec<String> = shown.iter().map(|(s, n, k)| format!("{s}{}  {n}  · {k}", " ".repeat(w.saturating_sub(s.width())))).collect();
    if hits.len() > 30 {
        out.push(format!("… and {} more — add words to narrow it down", hits.len() - 30));
    }
    Ok(out.join("\n"))
}

fn file(input: &str, output: &str) -> Result<String, String> {
    let (inp, out) = (files::parse_path(input), files::parse_path(output));
    let ext = |p: &std::path::Path| p.extension().map_or(String::new(), |e| e.to_string_lossy().into_owned());
    let a = files::of_path(&inp).ok_or(format!("unknown input type \".{}\"", ext(&inp)))?;
    let b = files::of_path(&out).ok_or(format!("unknown output type \".{}\"", ext(&out)))?;
    let backend = *files::backends(a, b)
        .first()
        .ok_or(format!("nothing on this computer converts {} to {} — install {}", files::FORMATS[a].name, files::FORMATS[b].name, files::suggest(a)))?;
    if out.exists() {
        return Err(format!("{} already exists — choose another name", files::display_path(&out)));
    }
    let saved = Settings::load().file_opts;
    let opts = files::options(a, b, backend)
        .into_iter()
        .map(|o| (o.key, saved.get(o.key).and_then(|v| o.choices.iter().position(|c| c == v)).unwrap_or(o.default)))
        .collect();
    let job = files::Job { input: inp, output: out, from: a, to: b, backend, opts, overwrite: false };
    let running = files::start(job);
    loop {
        match running.rx.recv() {
            Ok(files::Event::Done(r)) => {
                let d = r?;
                return Ok(format!("{} · {} · {:.1} s · via {}", files::display_path(&d.output), files::human_size(d.bytes), d.secs, backend.name()));
            }
            Ok(files::Event::Progress(_)) => {}
            Err(_) => return Err("the conversion stopped unexpectedly".into()),
        }
    }
}

fn translate(from: &str, to: &str, text: &str) -> Result<String, String> {
    let c = catalog();
    let cat = c.categories.iter().position(|k| k.id == "translate").unwrap_or(0);
    let code = |q: &str| -> Result<&'static str, String> {
        let u = c.exact(q, cat).or_else(|| c.lookup(q, Some(cat))).ok_or(format!("unknown language \"{q}\""))?;
        match c.units[u].kind {
            Kind::Lang(i) => Ok(catalog::lang::code(i)),
            _ => Err(format!("unknown language \"{q}\"")),
        }
    };
    let (from, to) = (code(from)?, code(to)?);
    if to == "auto" {
        return Err("the target language can't be auto".into());
    }
    let s = Settings::load();
    translate::translate(&translate::Request::new(&s, text, from, to)).map(|t| t.text)
}

/// `add step = 0.75 m`, `add "step (stp)" = 0.75 m`, `add pizza`.
fn add(words: &[String]) -> Result<String, String> {
    let all = words.join(" ");
    let (left, equals) = all.split_once('=').map_or((all.as_str(), ""), |(l, r)| (l, r));
    let left = left.trim();
    let (name, symbol) = match left.strip_suffix(')').and_then(|l| l.rsplit_once('(')) {
        Some((n, s)) => (n.trim(), s.trim()),
        None => (left, ""),
    };
    let unit = CustomUnit { name: name.into(), symbol: symbol.into(), equals: equals.trim().into() };
    let mut s = Settings::load();
    let editing = s.custom.iter().position(|u| u.id() == unit.id());
    s.custom = custom::apply(&s.custom, editing, &unit)?;
    s.save();
    let verb = if editing.is_some() { "Updated" } else { "Added" };
    Ok(match unit.equals.as_str() {
        "" => format!("{verb} {} — a new kind of unit. Next: consertion add slice = 1/8 {}", unit.name, unit.symbol()),
        e => format!("{verb} {}: 1 {} = {e}. Try: consertion 10 {} {}", unit.name, unit.symbol(), unit.symbol(), custom::split(e).1),
    })
}

fn remove(name: &str) -> Result<String, String> {
    let mut s = Settings::load();
    let n = name.trim().to_lowercase();
    let i = s.custom.iter().position(|u| u.name.to_lowercase() == n || u.symbol().to_lowercase() == n).ok_or(format!("you have no unit called \"{name}\" — see consertion mine"))?;
    s.custom = custom::remove(&s.custom, i)?;
    s.save();
    Ok(format!("Removed {name}"))
}

fn mine() -> String {
    let s = Settings::load();
    if s.custom.is_empty() {
        return "No units of your own yet. Add one: consertion add step = 0.75 m".into();
    }
    let c = Catalog::with_custom(&s.custom);
    s.custom
        .iter()
        .map(|u| {
            let kind = c.find(&u.id()).map_or("not working — it refers to a unit that's gone", |i| c.categories[c.units[i].category].name.trim_start_matches("✎ "));
            let sym = if u.symbol.trim().is_empty() { String::new() } else { format!(" ({})", u.symbol.trim()) };
            let def = if u.equals.is_empty() { "a new kind".to_string() } else { format!("= {}", u.equals) };
            format!("{}{sym} {def}  · {kind}", u.name)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

const SUBCOMMANDS: [&str; 8] = ["find", "file", "translate", "add", "mine", "remove", "completions", "help"];

/// `__complete <n> <words…>`: candidates for word `n` (1 = first after the program name).
/// Shells call this; see `completions`.
fn complete(args: &[String]) {
    let n: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(1).max(1);
    let mut words: Vec<String> = args.iter().skip(1).cloned().collect();
    words.resize(n.max(words.len()), String::new());
    let prefix = words[n - 1].to_lowercase();
    let starts = |s: &str| s.to_lowercase().starts_with(&prefix);
    let mut out: Vec<String> = Vec::new();
    let first = words[0].as_str();
    match (n, first) {
        (1, _) => out.extend(SUBCOMMANDS.iter().filter(|s| starts(s)).map(|s| s.to_string())),
        (2, "completions") => out.extend(["bash", "zsh", "fish", "powershell"].iter().filter(|s| starts(s)).map(|s| s.to_string())),
        (2 | 3, "translate") => out.extend(catalog::lang::LANGS.iter().map(|l| l.0.to_string()).filter(|s| starts(s))),
        (_, "remove" | "rm") => out.extend(Settings::load().custom.iter().map(|u| u.name.clone()).filter(|s| starts(s))),
        (_, "find" | "file" | "translate" | "add" | "mine" | "help" | "completions") => {}
        _ => {
            let c = catalog();
            let value = words[0].clone();
            let mut names = Vec::new();
            // The source: units that can read the typed value. The target: units of the same kind.
            let source = (n > 2).then(|| c.search(&words[1], None, |u| u.kind.can_parse() && u.kind.accepts(&value)).first().copied()).flatten();
            if n == 3 && "to".starts_with(&prefix) && !prefix.is_empty() {
                out.push("to".into());
            }
            // Everyday units first: km before kiloampere.
            let mut order: Vec<usize> = (0..c.units.len()).collect();
            order.sort_by_key(|&i| (!c.units[i].common, c.categories[c.units[i].category].rank));
            for i in order {
                let u = &c.units[i];
                let fits = match source {
                    Some(f) => u.category == c.units[f].category && i != f && u.kind.can_target() && c.pairs(f, i),
                    None => u.kind.can_parse() && u.kind.accepts(&value) && !matches!(u.kind, Kind::Calc(_) | Kind::File(_) | Kind::Lang(_) | Kind::NewCustom),
                };
                if fits && starts(&u.symbol) {
                    out.push(u.symbol.clone());
                } else if fits && !prefix.is_empty() && starts(&u.name) {
                    names.push(u.name.clone());
                }
            }
            out.append(&mut names);
        }
    }
    let mut seen = std::collections::HashSet::new();
    for o in out.into_iter().filter(|o| seen.insert(o.clone())).take(200) {
        say(&o);
    }
}

fn completions(shell: &str) -> Result<String, String> {
    Ok(match shell {
        "bash" => r#"# consertion completion for bash. Add to ~/.bashrc:  source <(consertion completions bash)
_consertion() {
    local cur=${COMP_WORDS[COMP_CWORD]}
    if [[ ${COMP_WORDS[1]} == file && $COMP_CWORD -gt 1 ]]; then
        COMPREPLY=($(compgen -f -- "$cur")); return
    fi
    local IFS=$'\n'
    COMPREPLY=($(consertion __complete "$COMP_CWORD" "${COMP_WORDS[@]:1:COMP_CWORD}" 2>/dev/null | while IFS= read -r l; do printf '%q\n' "$l"; done))
}
complete -F _consertion consertion"#
            .into(),
        "zsh" => r#"# consertion completion for zsh. Add to ~/.zshrc (after compinit):  source <(consertion completions zsh)
_consertion() {
    if [[ ${words[2]} == file && $CURRENT -gt 2 ]]; then _files; return; fi
    local -a items
    items=("${(@f)$(consertion __complete $((CURRENT-1)) "${(@)words[2,CURRENT]}" 2>/dev/null)}")
    compadd -a items
}
compdef _consertion consertion"#
            .into(),
        "fish" => r#"# consertion completion for fish. Save as ~/.config/fish/completions/consertion.fish
function __consertion_complete
    set -l words (commandline -opc) (commandline -ct)
    consertion __complete (math (count $words) - 1) $words[2..-1] 2>/dev/null
end
complete -c consertion -f -n 'not __fish_seen_subcommand_from file' -a '(__consertion_complete)'
complete -c consertion -F -n '__fish_seen_subcommand_from file'"#
            .into(),
        "powershell" | "pwsh" => r#"# consertion completion for PowerShell. Add to $PROFILE:
#   consertion completions powershell | Out-String | Invoke-Expression
Register-ArgumentCompleter -Native -CommandName consertion -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)
    $words = @($commandAst.CommandElements | Select-Object -Skip 1 | ForEach-Object { $_.ToString().Trim("'", '"') })
    if ($wordToComplete -eq '') { $words += '' }
    consertion __complete $words.Count @words | ForEach-Object {
        $text = if ($_ -match '\s') { "'$_'" } else { $_ }
        [System.Management.Automation.CompletionResult]::new($text, $_, 'ParameterValue', $_)
    }
}"#
        .into(),
        other => return Err(format!("no completion for \"{other}\" — use bash, zsh, fish or powershell")),
    })
}

#[cfg(test)]
mod tests {
    use super::engineering;

    #[test]
    fn engineering_values() {
        assert_eq!(engineering("4.7k").as_deref(), Some("4.7e3"));
        assert_eq!(engineering("4k7").as_deref(), Some("4.7e3"));
        assert_eq!(engineering("100n").as_deref(), Some("100e-9"));
        assert_eq!(engineering("4R7").as_deref(), Some("4.7e0"));
        assert_eq!(engineering("12"), None);
        assert_eq!(engineering("15:00"), None);
        assert_eq!(engineering("C#5"), None);
    }
}
