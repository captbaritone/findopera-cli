# A marker naming a recording that is not in the database

One of these two ids is not there. The plan is refused whole rather than
half-built: the destination below is empty, not holding the one folder that
could have been made.

Which is the point. A recording that cannot be resolved is not the same as
one that left the library, and only the second is a reason to remove
anything.

## Library

```tree
BillyBudd/findopera-75.txt
Nowhere/findopera-99999999.txt
```

## Config

```toml
template = "{{opera.title}}"
link = "copy"
```

## Run

```console
$ findopera organize ./library --write
```

<!-- Everything below is generated. UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```

findopera: recording 99999999 is not in the FindOpera database (from ./library/Nowhere/findopera-99999999.txt)
findopera: the plan has problems, and half a tree is worse than none — fix them first
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["75","99999999"]}
```

## Exit

```
2
```
