# The destination is inside the library

Same objection one level down: whatever is built here is inside what gets
walked, so the library would grow every time it was named.

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
destination = "{library}/named"
```

## Run

```console
$ findopera organize ./library --write
```
