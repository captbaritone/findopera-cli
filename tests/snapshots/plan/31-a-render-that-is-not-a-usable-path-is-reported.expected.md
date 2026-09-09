# A render that is not a usable path is reported

<!-- From 31-a-render-that-is-not-a-usable-path-is-reported.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/billy	Ambrosian Opera Chorus
```

## stderr

```

findopera: recording 10655 rendered "", which has no path segments (from ./library/sosarme/findopera-10655.txt)
findopera: no destination set, so these are only the folder names. Add one to build it:
    destination = "/path/to/named"
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["75","10655"]}
```

## Exit

```
1
```
