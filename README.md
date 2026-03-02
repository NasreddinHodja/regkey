# regkey

A background keystroke recorder for Wayland. Captures every keypress with its modifier state and active application, stores it in a local SQLite database, and generates reports to guide the design of custom keymaps.

## What it answers

- Which keys do I actually press, and how often?
- Which modifier combinations do I use per application?
- Which keys do I press in sequence most frequently?

## Requirements

- Linux with a Wayland compositor (Hyprland supported; Sway stub included)
- Rust toolchain
- Membership in the `input` group (or run with `sudo`)

```bash
sudo usermod -aG input $USER   # then log out and back in
```

## Install

```bash
cargo install --path .
```

## Usage

### Record

```bash
regkey record                    # start recording (Ctrl-C to stop)
regkey record --window 1000      # bigram time window in ms (default: 2000)
```

Run as a background service — see [Autostart](#autostart).

### Report

```bash
regkey report                              # full report, JSON (default)
regkey report --format text               # human-readable
regkey report --app kitty                 # filter to one app
regkey report --app kitty,emacs           # union of multiple apps
regkey report --top 20                    # limit to top 20 results
```

JSON output includes keys, apps, and bigrams in one document.
Text output renders bar charts with aligned columns.

### Bigrams

Key-sequence report — which keys you press one after another, within the recording window.

```bash
regkey bigrams                             # all bigrams, JSON
regkey bigrams --format text              # human-readable
regkey bigrams --app kitty                # filter by app
regkey bigrams --top 30
```

Example text output:

```
Global bigram report
──────────────────────────────────────────
 1.  j -> k          45  ████████████████████  kitty
 2.  ctrl+c -> v     32  █████████████         kitty
```

### Clear

```bash
regkey clear                    # delete all recorded data
regkey clear --app firefox      # delete only Firefox keystrokes
```

## Data

Stored at `~/.local/share/regkey/keystrokes.db` (respects `$XDG_DATA_HOME`).
Two tables: `keystrokes` and `bigrams`. SQLite WAL mode — safe to query while recording.

## Autostart

A systemd user service is provided in `contrib/regkey.service`.

```bash
cp contrib/regkey.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now regkey
journalctl --user -u regkey -f    # logs
```

## Architecture

```
main.rs          CLI entry point (clap)
cli.rs           Subcommand definitions
db.rs            SQLite schema, insert, query helpers
record.rs        evdev threads + window provider thread -> mpsc -> DB writer
report.rs        Keystroke report (JSON / text)
bigrams.rs       Bigram report (JSON / text)
window/
  mod.rs         WindowProvider trait + compositor detection
  hyprland.rs    Hyprland socket2 listener + socket1 initial-window query
  sway.rs        Sway stub
  null.rs        Fallback (no app context)
```

## License

MIT
