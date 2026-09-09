# Two distinct recordings that render alike are numbered

<!-- From 20-two-distinct-recordings-that-render-alike-are-numbered.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/a	Mozart/Don Giovanni (1)
./library/b	Mozart/Don Giovanni (2)
```

## stderr

```

findopera: 2 directories were numbered by walk order because no variant was declared; those numbers shift as the library changes. Write a word into each marker to fix them — mv './library/a/findopera-332.txt' './library/a/findopera-332 <word>.txt' — or pass --require-variants to make this an error.
findopera: no destination set, so these are only the folder names. Add one to build it:
    destination = "/path/to/named"
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["332","5876"]}
```

## Exit

```
0
```
