# The library is inside the destination

The other way round, and refused for the same reason from the other side —
the destination would contain the library it is naming.

## Recording 75

```json
{"opera": {"title": "Billy Budd"}}
```

## Library

```tree
BillyBudd/findopera-75.txt
```

## Toml

```toml
template = '''{{opera.title}}'''
destination = "{root}"
```

## Run

```console
$ findopera organize ./library --write
```
