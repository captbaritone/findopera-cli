# findopera

Render opera recording metadata from [FindOpera](https://findopera.com/)
through a template.

Give it a recording id and a template; it prints the result. That's the whole
tool — a naming primitive. What you do with the string is up to you.

```bash
$ findopera render 10655 -t '{{composer.lastName}}/{{opera.title}}/{{year}}'
Handel/Sosarme, Re di Media/2026

$ findopera render 75 10655 -t '{{opera.title}} ({{year}})'
Billy Budd (1967)
Sosarme, Re di Media (2026)
```

The id is the number in a findopera.com URL:
`https://findopera.com/recording/10655` → `10655`.

## Templates

A placeholder is `{{field}}`. Alternatives separated by `|` are tried left to
right, and a quoted literal serves as a last resort:

```
{{opera.englishTitle|opera.title}}/{{year|"n.d."}} - {{conductor.lastName}}
```

**A placeholder that resolves to nothing, with no fallback, is an error.**
Absent data is never silently dropped — you get a non-zero exit and a message
naming the placeholder, so a missing year can't quietly become `Handel//2026`.

```bash
$ findopera render 10655 -t '{{opera.englishTitle}}'
{{opera.englishTitle}} resolved to nothing for this recording — add a fallback,
e.g. {{…|"Unknown"}}
$ echo $?
1
```

Values are sanitized so a `/` inside a title can't introduce an unintended path
separator, and a rendered result can't be an absolute path or contain `..`.

## Finding out which fields exist

```bash
findopera fields                    # every field, with a description
findopera fields --example 10655    # what each one holds for a real recording
```

`--example` is the useful one. Many fields are empty for any given recording,
so seeing real values tells you where a fallback is needed:

```
$ findopera fields --example 10655
id                           10655
year                         2026
month                        —
orchestra                    Orchestre de l'Opéra Royal
chorus                       —
opera.title                  Sosarme, Re di Media
opera.englishTitle           —
opera.librettist             —
composer.lastName            Handel
conductor.lastName           Angioloni
…                            (— means absent)
```

Measured across all 10,744 recordings in the database: `opera.title`,
`composer.lastName` and `conductor.lastName` are 100% populated, `year` is
99.9%, `orchestra` 94%, `chorus` 81%, and `opera.englishTitle` only **19.7%**.

## Output

JSON when stdout is piped, plain text at a terminal. Override with `--format
json|text|ndjson`.

```bash
$ findopera render 10655 -t '{{composer.lastName}}/{{opera.title}}' --format json
{
  "template": "{{composer.lastName}}/{{opera.title}}",
  "results": [
    {
      "id": "10655",
      "rendered": "Handel/Sosarme, Re di Media",
      "segments": ["Handel", "Sosarme, Re di Media"]
    }
  ],
  "problems": []
}
```

stdout carries data, stderr carries messages — always.

## Agent-friendly

Built to the [Agent CLI Design Guide](https://github.com/Johnixr/agent-cli-guide):
noun-verb commands, long flags, structured output as a contract, TTY-aware
defaults, strict input validation, JSON errors with a code / suggestion /
`retryable`, and `findopera schema --all` to dump the command tree and template
fields as JSON.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | success |
| 1 | general error — including a placeholder that resolved to nothing |
| 2 | invalid arguments or template |
| 3 | a recording id is not in the database |
| 6 | the API was unreachable or errored — retryable |

## Install

```bash
cargo install --path .
```

## Tests

```bash
cargo test              # offline; no network
cargo test -- --ignored # exercises the live findopera.com API
```

## See also

The `full-linking-prototype` branch carries an earlier, larger version that
also scanned directories for `findopera.com/recording/<id>.txt` marker files
and built a tree of symlinks from them. It works, but it was more machinery
than this needed to be.
