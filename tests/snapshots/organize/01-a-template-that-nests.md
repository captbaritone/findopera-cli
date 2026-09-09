# A template that turns a folder into a folder of variants

Built flat first, then the template puts the rip one level down. The old
folder is no longer wanted, and the new one is inside it.

Removals are recursive, so building before removing deleted what had just
been made — and still counted it as built. The summary and the destination
below are the two halves that have to agree; reading either alone would have
called this run a success.

## Library

```tree
BillyBudd/findopera-75.txt
BillyBudd/track01.flac
```

## Config

```toml
link = "copy"
```

## Recordings

```
75
```

## Run

```console
$ findopera organize ./library --write -t '{{opera.title}}'
$ findopera organize ./library --write -t '{{opera.title}}/audio'
```

<!-- Everything below is generated. UPDATE_EXPECT=1 cargo test -->

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
