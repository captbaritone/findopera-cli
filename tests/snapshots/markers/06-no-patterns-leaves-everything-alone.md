# No patterns leaves everything alone

`ignore` is empty unless a settings file says otherwise, so nothing is
skipped for having a name somebody once decided was uninteresting. The
`@eaDir` that the other case skips is found here, because nothing was asked
for.

## Recording 75

```json
{"opera": {"title": "Billy Budd"}}
```

## Library

```tree
a/@eaDir/findopera-75.txt
```

## Toml

```toml
template = '''{{opera.title}}'''
```

## Run

```console
$ findopera organize ./library --tabs
```
