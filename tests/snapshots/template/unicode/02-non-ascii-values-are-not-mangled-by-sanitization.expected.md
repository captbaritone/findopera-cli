# Non ascii values are not mangled by sanitization

<!-- From 02-non-ascii-values-are-not-mangled-by-sanitization.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/one	Les Troyens à Carthage
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
