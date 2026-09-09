# A copy is a file of its own

<!-- From 02-a-copy-is-its-own-file.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write
+ ./library/BillyBudd  Billy Budd
$ append './named/Billy Budd/track01.flac' ' — written through the destination'
$ show ./library/BillyBudd/track01.flac
BillyBudd/track01.flac
$ show './named/Billy Budd/track01.flac'
BillyBudd/track01.flac — written through the destination
```

## stderr

```
$ findopera organize ./library --write
findopera: building a copy of every file in ./named
findopera: 1 built, 0 already there, 0 left alone
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
```

## Exit

```
0
```
