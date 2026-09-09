# A destination, and a way of getting there

Both settings are read, and both are said out loud before anything is
touched. The destination is named because nothing on the command line names
it, and the way of getting there is named because a hard link and a copy are
very different things to have done to a disk — worth stating before doing,
not after.

That is a better thing to know than that the two fields hold what was written
in them.

## Recording 332

```json
{"opera": {"title": "Don Giovanni"}}
```

## Library

```tree
DonGiovanni/findopera-332.txt
```

## Toml

```toml
template = '''{{opera.title}}'''
destination = "{destination}"
link = "hardlink"
```

## Run

```console
$ findopera organize ./library --tabs
```
