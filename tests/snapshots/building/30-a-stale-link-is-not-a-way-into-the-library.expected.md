# A link left by an earlier run is not a way into the library

<!-- From 30-a-stale-link-is-not-a-way-into-the-library.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write
+ ./library/BillyBudd  Billy Budd/rip
$ rm './named/Billy Budd'
$ link ./library/BillyBudd './named/Billy Budd'
$ findopera organize ./library --write
! ./library/BillyBudd  Billy Budd/rip
$ show ./library/BillyBudd/findopera-75.txt
BillyBudd/findopera-75.txt
$ show ./library/BillyBudd/rip
<cannot read ./library/BillyBudd/rip: entity not found>
```

## stderr

```
$ findopera organize ./library --write
findopera: building a link to each folder in ./named
findopera: 1 built, 0 already there, 0 left alone
$ findopera organize ./library --write
findopera: building a link to each folder in ./named
findopera: ./named/Billy Budd/rip
    ./named/Billy Budd is a link to somewhere else, so building this inside it would write outside ./named — most likely into the library itself
findopera: 0 built, 0 already there, 1 left alone
```

## Destination

```tree
Billy Budd -> …/BillyBudd
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
1
```
