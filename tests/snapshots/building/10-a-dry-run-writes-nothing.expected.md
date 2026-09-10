# A dry run writes nothing, and removes nothing

<!-- From 10-a-dry-run-writes-nothing.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write
+ ./library/BillyBudd  Billy Budd
$ findopera organize ./library --dry-run -t '{{opera.title}} {{id}}'
- Billy Budd
+ ./library/BillyBudd  Billy Budd 75
```

## stderr

```
$ findopera organize ./library --write
findopera: building a copy of every file in ./named
findopera: 1 built, 0 already there, 0 left alone
$ findopera organize ./library --dry-run -t '{{opera.title}} {{id}}'
findopera: would build a copy of every file in ./named
findopera: 1 to build, 0 already there, 0 left alone, 1 to remove
findopera: nothing was written. To build it, run:
    findopera organize ./library --config ./findopera.toml --write
```

## Destination

```tree
Billy Budd/
Billy Budd/findopera-75.txt
Billy Budd/track01.flac
```

## Requests

```
Recordings
  {"ids":["75"]}
Recordings
  {"ids":["75"]}
```

## Exit

```
0
0
```
