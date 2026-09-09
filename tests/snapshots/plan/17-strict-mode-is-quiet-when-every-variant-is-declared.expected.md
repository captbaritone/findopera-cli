# Strict mode is quiet when every variant is declared

<!-- From 17-strict-mode-is-quiet-when-every-variant-is-declared.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/rips/flac	Don Giovanni [332] (flac)
./library/rips/mp3	Don Giovanni [332] (mp3)
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
