# A declared and an undeclared variant do not clash

<!-- From 12-a-declared-and-an-undeclared-variant-do-not-clash.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/rips/flac	Don Giovanni [332] (flac)
./library/rips/other	Don Giovanni [332]
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
  {"ids":["332","332"]}
```

## Exit

```
0
```
