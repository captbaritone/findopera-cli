# Folders to leave alone

A NAS leaves its own folders lying about, and a library halfway through a
download should not be named yet. Both are ignored by pattern.

The proof is which folders reach the listing: `@eaDir` and `Incomplete` hold
markers exactly like the real one, and neither is planned. `**/Artwork/**`
reaches one nested further down.

## Toml

```toml
template = '''{{opera.title}}'''
ignore = ["@eaDir", "Incomplete", "**/Artwork/**"]
```

## Library

```tree
DonGiovanni/findopera-332.txt
@eaDir/findopera-75.txt
Incomplete/findopera-10655.txt
Boxset/Artwork/findopera-5000.txt
```

## Run

```console
$ findopera organize ./library --tabs
```
