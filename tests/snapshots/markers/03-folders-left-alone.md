# Folders left alone

Two kinds of pattern, and the difference between them is the point.

A bare name matches that folder wherever it turns up, at any depth — which is
what `@eaDir` needs, since a Synology leaves one beside every folder it
indexes. A pattern with a `/` in it is a path, anchored at the top of the
library: `Incomplete/**` skips the one being downloaded into, and leaves a
folder of the same name further down alone.

A skipped folder is never looked inside, so a marker below one is not merely
unlisted — it is never read.

Two survive: the one outside every pattern, and the `Incomplete` that is not
at the top.

## Recording 75

```json
{"opera": {"title": "Billy Budd"}}
```

## Recording 10655

```json
{"opera": {"title": "Sosarme"}}
```

## Recording 1721

```json
{"opera": {"title": "Aida"}}
```

## Library

```tree
keep/findopera-75.txt
@eaDir/findopera-1721.txt
keep/@eaDir/findopera-1721.txt
Incomplete/findopera-1721.txt
Boxes/Incomplete/findopera-10655.txt
skip/inner/deeper/findopera-1721.txt
```

## Toml

```toml
template = '''{{opera.title}}'''
ignore = ["@eaDir", "Incomplete/**", "skip"]
```

## Run

```console
$ findopera organize ./library --tabs
```
