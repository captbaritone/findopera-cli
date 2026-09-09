# The destination is the library

Building the names among the things being named. The next run would walk
what this one built, and there would be no telling the two apart.

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
destination = "{library}"
```

## Run

```console
$ findopera organize ./library --write
```
