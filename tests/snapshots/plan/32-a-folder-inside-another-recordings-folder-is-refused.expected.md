# A folder inside another recordings folder is refused

<!-- From 32-a-folder-inside-another-recordings-folder-is-refused.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/rips/flac	Don Giovanni/flac
./library/rips/plain	Don Giovanni
```

## stderr

```

findopera: "Don Giovanni/flac" would be built inside "Don Giovanni", which is itself a recording's folder:
    ./library/rips/plain/findopera-332.txt
    ./library/rips/flac/findopera-332 flac.txt
    ^ they cannot both be built — with links, the second would be written through the first, into the library itself. This comes from a template whose optional group contains a `/`, so a folder that drops it encloses one that does not.
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
