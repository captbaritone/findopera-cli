# findopera

Organize opera recordings into a canonical directory tree, using the
[FindOpera](https://findopera.com/) database for the metadata.

## How it works

FindOpera serves a plain-text card for every recording at
`https://findopera.com/recording/<id>.txt`:

```
                    SOSARME, RE DI MEDIA

           by George Frideric Handel (1685-1759)

                         -= Cast =-
    Sosarme ......................... Rémy Brès-Feuillet
    Elmira ............................... Sarah Charles
    ...

Conductor: Marco Angioloni
Recorded: 2026
Orchestra: Orchestre de l'Opéra Royal


           https://findopera.com/recording/10655
```

Drop that file into the directory holding the recording. `findopera` walks your
music directories, finds those markers, looks the recordings up in the FindOpera
API, and builds a tree of symlinks named by a template you supply.

Your original directories are never moved or modified.

```
~/Music/Opera/                          ~/Opera/            (the canonical tree)
  billy_budd_britten_67/                  Britten/
    Billy Budd.txt          ─────────▶      Billy Budd/
    disc1.flac                                1967 - Britten -> ~/Music/Opera/billy_budd_britten_67
  sosarme_2026/                           Handel/
    Sosarme…-2026-Angioloni.txt ──────▶     Sosarme, Re di Media/
    disc1.flac                                2026 - Angioloni -> ~/Music/Opera/sosarme_2026
```

A directory may hold several markers — a box set covering several operas gets a
link per recording, all pointing at the same directory.

## Usage

```bash
# What did it find?
findopera marker list --source ~/Music/Opera

# Preview the tree (writes nothing)
findopera library plan \
  --source ~/Music/Opera \
  --destination ~/Opera \
  --template '{{composer.lastName}}/{{opera.title}}/{{year}} - {{conductor.lastName}}'

# Build it
findopera library sync \
  --source ~/Music/Opera \
  --destination ~/Opera \
  --template '{{composer.lastName}}/{{opera.title}}/{{year}} - {{conductor.lastName}}' \
  --apply
```

`library sync` previews by default; nothing is written until `--apply`.

## Templates

A placeholder is `{{field}}`. Alternatives separated by `|` are tried left to
right, and a quoted literal serves as a final fallback:

```
{{opera.englishTitle|opera.title}}/{{year|"n.d."}} - {{conductor.lastName|"unknown"}}
```

`/` in the template is a directory separator; a `/` *inside* a value (an opera
title, say) is replaced so it can't create an unintended directory level.

If a placeholder resolves to nothing and has no literal fallback, that recording
is reported as a problem rather than silently producing a malformed path.

## Finding out which fields exist

```bash
findopera library fields                      # every field, with a description
findopera library fields --example 10655      # what each one holds for a real recording
findopera library fields --format json        # same, machine-readable
```

`--example` is the useful one. Plenty of fields are empty for any given
recording, and an unresolved placeholder is an error, so seeing real values
tells you where you need a `|` fallback:

```
$ findopera library fields --example 10655
id                           10655
year                         2026
month                        —
orchestra                    Orchestre de l'Opéra Royal
chorus                       —
opera.title                  Sosarme, Re di Media
opera.englishTitle           —
opera.librettist             —
opera.language               —
composer.lastName            Handel
conductor.lastName           Angioloni
…                            (— means absent)
```

`findopera schema --all` carries the same list under `templateFields`.

## The destination is rebuilt, not merged

Every `--apply` wipes the destination and recreates it, so the tree always
matches your markers exactly — rename a directory or delete a marker and the
stale link disappears.

To make that safe, an applied destination gets a `.findopera-library.json` stamp
file. A destination that is non-empty and **not** stamped is refused; pass
`--force` to override. Point `--destination` at a directory you own and keep
nothing in it by hand.

## Agent-friendly

Built to the [Agent CLI Design Guide](https://github.com/Johnixr/agent-cli-guide):

- **Noun-verb commands** — `library sync`, `recording get`, `marker list`
- **Long flags** on everything; short forms are extras
- **stdout is data, stderr is messages** — always
- **TTY-aware** — JSON when piped, a table at a terminal; `NO_COLOR` respected
- **Dry-run by default** for anything that writes
- **Semantic exit codes** (below)
- **Strict validation** — unknown template fields are rejected before any I/O
- **`findopera schema --all`** dumps the command tree as JSON
- **Errors are JSON** on stderr with a code, a suggestion, and `retryable`

### Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | general error |
| 2 | invalid arguments or template |
| 3 | a marker names a recording not in the database |
| 4 | permission denied reading or writing a path |
| 5 | destination not managed by findopera, or a path claimed by two recordings |
| 6 | API unreachable or errored — retryable |
| 10 | plan produced and safe to run with `--apply` |

## Install

```bash
cargo install --path .
```
