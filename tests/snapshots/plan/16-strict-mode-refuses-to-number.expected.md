# Strict mode refuses to number

<!-- From 16-strict-mode-refuses-to-number.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/elsewhere	Billy Budd [75]
./library/rips/a	Don Giovanni [332] (1)
./library/rips/b	Don Giovanni [332] (2)
```

## stderr

```

findopera: 2 directories would take the same name and were numbered by walk order. Write a word into each marker instead — one you choose, which `{{variant}}` then picks up:
    mv './library/rips/a/findopera-332.txt' './library/rips/a/findopera-332 <word>.txt'
    mv './library/rips/b/findopera-332.txt' './library/rips/b/findopera-332 <word>.txt'
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
1
```
