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
