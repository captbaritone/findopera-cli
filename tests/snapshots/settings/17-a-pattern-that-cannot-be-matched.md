# A pattern that cannot be matched

Caught when the settings are read, naming the pattern, rather than silently
matching nothing on every folder in the library.

## Toml

```toml
template = '''{{opera.title}}'''
ignore = ["["]
```

## Library

```tree
DonGiovanni/findopera-332.txt
```

## Run

```console
$ findopera organize ./library
```
