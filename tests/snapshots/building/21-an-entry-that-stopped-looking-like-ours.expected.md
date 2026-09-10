# An entry that stopped looking like ours is left alone

<!-- From 21-an-entry-that-stopped-looking-like-ours.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write
+ ./library/BillyBudd  Billy Budd
$ rm './named/Billy Budd'
$ write './named/Billy Budd' 'a file now, whatever it was before'
$ findopera organize ./library --write -t '{{opera.title}} {{id}}'
+ ./library/BillyBudd  Billy Budd 75
$ show './named/Billy Budd'
a file now, whatever it was before
```

## stderr

```
$ findopera organize ./library --write
findopera: building a copy of every file in ./named
findopera: 1 built, 0 already there, 0 left alone
$ findopera organize ./library --write -t '{{opera.title}} {{id}}'
findopera: building a copy of every file in ./named
findopera: ./named/Billy Budd
    recorded as a folder, and now a file — left alone
findopera: 1 built, 0 already there, 0 left alone
```

## Destination

```tree
Billy Budd
Billy Budd 75/
Billy Budd 75/findopera-75.txt
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
