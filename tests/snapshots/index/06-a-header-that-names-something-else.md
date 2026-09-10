# A header that names something else

A header is written once, above everything, so a recording's fields mean
nothing to it — there is no recording it could be about. The two it may name
are the two nobody can keep right by hand.

`{{conductor.lastName}}` is a field, and not one of them.

## Library

```tree
Mozart/findopera-332.txt
```

## Toml

```toml
template = '''{{opera.title}}'''

[index]
file = "list.txt"
header = '''Everything conducted by {{conductor.lastName}}.'''
template = '''{{opera.title}} (findopera {{id}})'''
```

## Run

```console
$ findopera organize ./library
```
