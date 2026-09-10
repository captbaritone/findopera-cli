# A template without variant cannot be numbered

<!-- From 15-a-template-without-variant-cannot-be-numbered.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/rips/a	Don Giovanni [332]
./library/rips/b	Don Giovanni [332]
```

## stderr

```

findopera: 2 directories want the name "Don Giovanni [332]":
    ./library/rips/a/findopera-332.txt
    ./library/rips/b/findopera-332.txt
    ^ the template never asks for `{{variant}}`; add it to give them different names
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
