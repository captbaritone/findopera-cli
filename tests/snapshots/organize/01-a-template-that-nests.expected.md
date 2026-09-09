# A template that turns a folder into a folder of variants

<!-- From 01-a-template-that-nests.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write -t '{{opera.title}}'
+ ./library/BillyBudd  Billy Budd
$ findopera organize ./library --write -t '{{opera.title}}/audio'
+ ./library/BillyBudd  Billy Budd/audio
- Billy Budd
```

## stderr

```
$ findopera organize ./library --write -t '{{opera.title}}'
findopera: building a copy of every file in ./named
findopera: 1 built, 0 already there, 0 left alone
$ findopera organize ./library --write -t '{{opera.title}}/audio'
findopera: building a copy of every file in ./named
findopera: 1 built, 0 already there, 0 left alone, 1 removed
```

## Destination

```tree
Billy Budd/
Billy Budd/audio/
Billy Budd/audio/findopera-75.txt
Billy Budd/audio/track01.flac
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
