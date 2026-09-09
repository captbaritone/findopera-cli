# A destination, and a way of getting there

Both settings are read, and the proof is that they are checked against each
other before anything is touched. A hard link is a second name for a file on
the disk it already lives on, so naming a destination on another disk and
asking for hard links is a contradiction — and the refusal names both
settings, and what to put instead.

That is a better thing to know than that the two fields hold what was
written in them.

## Toml

```toml
template = '''{{opera.title}}'''
destination = "/Volumes/Opera/named"
link = "hardlink"
```

## Library

```tree
DonGiovanni/findopera-332.txt
```

## Run

```console
$ findopera organize ./library --tabs
```
