# A list template that does not parse

Checked when the settings are read, not at the end of a build — a template
that will never render is a mistake in this file, and finding out after the
tree is built is finding out too late.

`{{singers.surnames}}` is not a field. Nothing is walked, nothing is fetched,
and the run stops.

## Library

```tree
Mozart/findopera-332.txt
```

## Toml

```toml
template = '''{{opera.title}}'''

[index]
file = "list.txt"
template = '''{{opera.title}} — {{singers.surnames}}'''
```

## Run

```console
$ findopera organize ./library
```
