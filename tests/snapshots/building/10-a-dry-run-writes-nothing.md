# A dry run writes nothing, and removes nothing

The first run builds. The second is asked for a name the first did not make,
which would mean building one folder and removing another — and being a dry
run, does neither. The destination afterwards is exactly what the first run
left.

Saying `--dry-run` rather than leaving `--write` off is the same thing said
out loud, so a script can state its intention instead of relying on the
absence of a flag.

## Recording 75

```json
{"opera": {"title": "Billy Budd"}}
```

## Library

```tree
BillyBudd/findopera-75.txt
BillyBudd/track01.flac
```

## Config

```toml
template = "{{opera.title}}"
link = "copy"
```

## Run

```console
$ findopera organize ./library --write
$ findopera organize ./library --dry-run -t '{{opera.title}} {{id}}'
```
