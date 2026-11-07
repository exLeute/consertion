# Consertion

A universal converter for the terminal. Units, currencies, time zones, clothing
and paper sizes, cooking measures, music notes, colors, number bases, text
encodings, files, and a few dozen handy calculators. One small binary. Works
offline, except for currency rates and translation.

Type what you're looking for, pick a target, and type a value. Both sides are
editable, and everything else in the same category updates below as you type.

## Install

**Linux / macOS**

```sh
curl -fsSL https://raw.githubusercontent.com/exLeute/consertion/master/install.sh | sh
```

It asks where to put the binary (`~/.local/bin` by default, or `/usr/local/bin`
for everyone) and offers to add that folder to your PATH. Press Enter through it
for the defaults. It's a single file, with nothing else to set up.

**Windows**: grab `consertion-x86_64-pc-windows-msvc.zip` from
[Releases](https://github.com/exLeute/consertion/releases/latest), unzip, and
run `consertion.exe` in Windows Terminal.

You can also download any binary from
[Releases](https://github.com/exLeute/consertion/releases/latest) by hand. On
macOS, if a browser download gets blocked, run
`xattr -d com.apple.quarantine consertion`.

**From source** (Rust 1.88 or newer):

```sh
git clone https://github.com/exLeute/consertion && cd consertion
./install.sh                 # builds it and installs it, no download
```

Run inside a clone, `install.sh` builds from source (or reuses a binary you
already built) instead of downloading. You can also skip it:
`cargo build --release` leaves the binary in `target/release/consertion`, and
`cargo install --git https://github.com/exLeute/consertion` does everything in
one go.

## Use it

Run `consertion` to open the app. Press `F1` for every shortcut. The ones you'll
actually use:

| Key      |                                                |
| -------- | ---------------------------------------------- |
| type     | search everything                              |
| `Enter`  | pick / copy the result                         |
| `Tab`    | move between panels                            |
| `Esc`    | step back (clear search → show all → back)     |
| `Ctrl+S` | swap the two sides                             |
| `Ctrl+F` | favorite                                       |
| `F2`     | settings (themes, precision, translator keys…) |

Or skip the app:

```sh
consertion 5 mi km                  # 8.04672
consertion 5 mi                     # 5 miles in every common length unit
consertion 15:00 kyiv to new york   # Sat 2026-11-07 08:00 (UTC−5)
consertion 100 usd eur
consertion 1 "cup of flour" g       # 125
consertion C#5 note Hz              # 554.365262
consertion 34C "US bra" "EU bra"    # 75C
consertion find pressure            # what's available
consertion file photo.heic photo.jpg
consertion translate auto de "good morning"
```

Tab completion knows unit names and only suggests targets that make sense:

```sh
source <(consertion completions bash)       # in ~/.bashrc
source <(consertion completions zsh)        # in ~/.zshrc, after compinit
consertion completions fish > ~/.config/fish/completions/consertion.fish
consertion completions powershell | Out-String | Invoke-Expression   # in $PROFILE
```

## What's in it

- **Everyday**: length, weight, volume, temperature, area, speed, time, dates
  and timestamps, currencies (live rates, cached for offline use), time zones
  for 100+ cities, cooking (cups and spoons of 23 ingredients ↔ grams), fuel
  economy, map coordinates (decimal, DMS, plus codes, geohash).
- **Sizes**: shoes, rings, women's and men's clothing, bras, paper (A4 → mm,
  inches, pixels at any dpi).
- **Music & photo**: note names ↔ Hz and MIDI, tempo ↔ delay times, intervals,
  sample and frame counts, exposure stops and ND filters.
- **Data & text**: bytes and bitrates, number bases and Roman numerals, colors
  (hex, RGB, HSL, OKLCH…), Base64, URL encoding, hashes, Unix permissions.
- **Science**: pressure, energy, power, force, electrical units, radiation,
  light, chemistry (molar mass from a formula), and more.
- **Calculators**: percentages, loans and interest, tips, BMI and calories,
  Ohm's law, exposure and depth of field, song length, and about 80 others.
- **Files**: images are built in (PNG, JPEG, WebP, GIF, BMP, TIFF, ICO…), and so
  are JSON / YAML / TOML / CSV, text encodings, Markdown → HTML and subtitles.
  Audio, video and documents work when [ffmpeg](https://ffmpeg.org),
  [ImageMagick](https://imagemagick.org), [LibreOffice](https://libreoffice.org)
  or [pandoc](https://pandoc.org) are installed. The app only offers what your
  machine can actually do.
- **Translation**: MyMemory works out of the box. DeepL or LibreTranslate work
  if you add your own key.

Math is exact (fractions, not floats), so `0.1 + 0.2` is `0.3` and huge numbers
stay precise. Every number field takes arithmetic: `2*(3+4)`, `1/8`, `1e300`.

## Your own units

In the app, open **✎ My Converters → New custom converter…**, or just search for
something that doesn't exist yet and press Enter on "Create…". Give it a name
and say what it equals:

```
step  = 0.75 m      → converts to every length unit
mug   = 350 ml      → every volume unit
pizza               → a new kind of thing
slice = 1/8 pizza
```

The same from the command line:

```sh
consertion add step = 0.75 m
consertion add "workday (wd)" = 8 h   # optional symbol in brackets
consertion mine                       # list them
consertion remove step
```

`Ctrl+E` edits one of yours, and `Ctrl+D` twice deletes it. Renaming a unit
keeps the ones built on it working. Loops and duplicates are refused.

## Where things live

Settings and your units are in `settings.json`, in your config folder
(`~/.config/consertion` on Linux, `~/Library/Application Support/consertion` on
macOS, `%APPDATA%\consertion` on Windows). The currency cache sits in the
matching cache folder. Delete either one to start fresh.

The app only goes online to fetch currency rates (from the free
[currency-api](https://github.com/fawazahmed0/exchange-api)) and to translate.
Nothing else.

## Hacking on it

`cargo test` runs everything. Units live in `src/catalog/`, one file per topic,
and most additions are a single table row. `src/catalog/mod.rs` has notes on
adding new kinds of values. To cut a release, push a tag like `v0.2.0`. GitHub
Actions builds Linux, macOS and Windows binaries and attaches them to the
release.

MIT licensed.
