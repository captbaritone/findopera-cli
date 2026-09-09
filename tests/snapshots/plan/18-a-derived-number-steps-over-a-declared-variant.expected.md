# A derived number steps over a declared variant

<!-- From 18-a-derived-number-steps-over-a-declared-variant.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/a	Don Giovanni [332] (2)
./library/b	Don Giovanni [332] (1)
./library/c	Don Giovanni [332] (3)
```

## stderr

```

findopera: 2 directories were numbered by walk order because no variant was declared; those numbers shift as the library changes. Write a word into each marker to fix them — mv './library/b/findopera-332.txt' './library/b/findopera-332 <word>.txt' — or pass --require-variants to make this an error.
findopera: no destination set, so these are only the folder names. Add one to build it:
    destination = "/path/to/named"
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["332","332","332"]}
```

## Exit

```
0
```
