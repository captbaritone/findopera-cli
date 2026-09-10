# Nothing outside the record is ever removed

<!-- From 20-nothing-outside-the-record-is-removed.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write
+ ./library/BillyBudd  Billy Budd
$ write './named/My Own Mixes/track.flac' 'nobody asked findopera about this'
$ findopera organize ./library --write -t '{{opera.title}} {{id}}'
- Billy Budd
+ ./library/BillyBudd  Billy Budd 75
$ show './named/My Own Mixes/track.flac'
nobody asked findopera about this
```

## stderr

```
$ findopera organize ./library --write
findopera: building a copy of every file in ./named
findopera: 1 built, 0 already there, 0 left alone
$ findopera organize ./library --write -t '{{opera.title}} {{id}}'
findopera: building a copy of every file in ./named
findopera: 1 built, 0 already there, 0 left alone, 1 removed
```

## Destination

```tree
Billy Budd 75/
Billy Budd 75/findopera-75.txt
My Own Mixes/
My Own Mixes/track.flac
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
