# Non ascii text passes through untouched

<!-- From 01-non-ascii-text-passes-through-untouched.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/one	Dvořák — Rusalka
```

## stderr

```
findopera: no destination set, so these are only the folder names. Add one to build it:
    destination = "/path/to/named"
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["1"]}
```

## Exit

```
0
```
