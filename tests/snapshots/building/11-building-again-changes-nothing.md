# Building again changes nothing

The second run finds everything where the first put it and says so — nothing
built, one already there. A run that is idempotent is what makes it safe to
put in a cron job.

A file added to the destination by hand is left alone too: the mirroring
modes fill a folder rather than replacing it, so what is already inside
survives being built into again.

## Recording 75

```json
{"opera": {"title": "Billy Budd"}}
```

## Library

```tree
BillyBudd/findopera-75.txt
BillyBudd/track01.flac
```

## Config

```toml
template = "{{opera.title}}"
link = "copy"
```

## Run

```console
$ findopera organize ./library --write
$ write './named/Billy Budd/notes-of-my-own.txt' 'put here by hand'
$ findopera organize ./library --write
$ show './named/Billy Budd/notes-of-my-own.txt'
```
