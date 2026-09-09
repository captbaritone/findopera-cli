# One recording named twice in one folder

<!-- From 04-one-recording-named-twice.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/a/b/c/d/e	Sosarme
./library/twice	Don Giovanni (flac)
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
  {"ids":["10655","332"]}
```

## Exit

```
0
```
