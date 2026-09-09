# An id not in the database is reported

<!-- From 30-an-id-not-in-the-database-is-reported.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
./library/elsewhere	Billy Budd
```

## stderr

```

findopera: recording 999999 is not in the FindOpera database (from ./library/somewhere/findopera-999999.txt)
findopera: no destination set, so these are only the folder names. Add one to build it:
    destination = "/path/to/named"
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["75","999999"]}
```

## Exit

```
1
```
