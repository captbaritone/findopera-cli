# A template alone is enough

Everything except the template has a default, so this is the smallest
settings file that works — which is what someone writing their first one
needs to be true.

The rest of the defaults are not pinned here. A default that cannot change
what anyone sees is not worth a case: `link` shows up only when something is
built, and the two that gate behaviour — numbering, and following links —
are shown where that behaviour is.

## Toml

```toml
template = '''{{opera.title}}'''
```

## Library

```tree
DonGiovanni/findopera-332.txt
```

## Run

```console
$ findopera organize ./library --tabs
```
