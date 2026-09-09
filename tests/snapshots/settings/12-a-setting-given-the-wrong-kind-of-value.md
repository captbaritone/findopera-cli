# A setting given the wrong kind of value

## Toml

```toml
template = '''{{opera.title}}'''
require-variants = "yes"
```

## Library

```tree
DonGiovanni/findopera-332.txt
```

## Run

```console
$ findopera organize ./library
```
