# A quoted literal may contain an escaped quote

<!-- From 06-a-quoted-literal-may-contain-an-escaped-quote.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/one	the "lost" opera
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
