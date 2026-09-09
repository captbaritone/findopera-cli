# The same variant declared twice cannot be numbered

<!-- From 14-the-same-variant-declared-twice-cannot-be-numbered.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/rips/a	Don Giovanni [332] (flac)
./library/rips/b	Don Giovanni [332] (flac)
```

## stderr

```

findopera: 2 directories want the name "Don Giovanni [332] (flac)":
    ./library/rips/a/findopera-332 flac.txt
    ./library/rips/b/findopera-332 flac.txt
    ^ these markers declare the same variant; give one a different word
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
1
```
