# Building again changes nothing

<!-- From 11-building-again-changes-nothing.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write
+ ./library/BillyBudd  Billy Budd
$ write './named/Billy Budd/notes-of-my-own.txt' 'put here by hand'
$ findopera organize ./library --write
  ./library/BillyBudd  Billy Budd
$ show './named/Billy Budd/notes-of-my-own.txt'
put here by hand
```

## stderr

```
$ findopera organize ./library --write
findopera: building a copy of every file in ./named
findopera: 1 built, 0 already there, 0 left alone
$ findopera organize ./library --write
findopera: building a copy of every file in ./named
findopera: 0 built, 1 already there, 0 left alone
```

## Destination

```tree
Billy Budd/
Billy Budd/findopera-75.txt
Billy Budd/notes-of-my-own.txt
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
