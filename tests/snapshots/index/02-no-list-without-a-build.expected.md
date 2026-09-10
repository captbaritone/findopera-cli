# No list without a build

<!-- From 02-no-list-without-a-build.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
+ ./library/Britten  Billy Budd
```

## stderr

```
findopera: would build a copy of every file in ./named
findopera: 1 to build, 0 already there, 0 left alone
findopera: nothing was written. To build it, run:
    findopera organize ./library --config ./findopera.toml --write
```

## Destination

```tree
```

## Index

```text title="./named/00 - What is in here.txt"
<not written: entity not found>
```

## Requests

```
Recordings
  {"ids":["75"]}
```

## Exit

```
0
```
