# A slash inside a value cannot create a directory level

<!-- From 01-a-slash-inside-a-value-cannot-create-a-directory-level.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/one	Mascagni/Cavalleria rusticana - Pagliacci
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
