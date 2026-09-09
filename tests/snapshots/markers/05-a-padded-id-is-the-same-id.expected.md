# A padded id is the same id

<!-- From 05-a-padded-id-is-the-same-id.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/padded	Billy Budd
./library/plain	Billy Budd
```

## stderr

```

findopera: 2 directories want the name "Billy Budd":
    ./library/padded/findopera-075.txt
    ./library/plain/findopera-75.txt
    ^ the template has no `{{variant}}` for them to differ in
findopera: no destination set, so these are only the folder names. Add one to build it:
    destination = "/path/to/named"
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["75","75"]}
```

## Exit

```
1
```
