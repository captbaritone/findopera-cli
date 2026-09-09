# Undeclared variants are numbered only when they clash

<!-- From 13-undeclared-variants-are-numbered-only-when-they-clash.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/elsewhere	Billy Budd [75]
./library/rips/a	Don Giovanni [332] (1)
./library/rips/b	Don Giovanni [332] (2)
```

## stderr

```

findopera: 2 directories were numbered by walk order because no variant was declared; those numbers shift as the library changes. Write a word into each marker to fix them — mv './library/rips/a/findopera-332.txt' './library/rips/a/findopera-332 <word>.txt' — or pass --require-variants to make this an error.
findopera: no destination set, so these are only the folder names. Add one to build it:
    destination = "/path/to/named"
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["75","332","332"]}
```

## Exit

```
0
```
