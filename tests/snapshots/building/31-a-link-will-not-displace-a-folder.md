# A link will not displace what is already there

Unlike the mirroring modes, this one cannot merge. A copy fills a folder that
already exists; a link and a folder cannot share one name at all. So when
something is in the way it stops and says what, and leaves it exactly as it
found it.

## Requires

```
unix
```

## Recording 75

```json
{"opera": {"title": "Billy Budd"}}
```

## Library

```tree
BillyBudd/findopera-75.txt
```

## Config

```toml
template = "{{opera.title}}"
link = "symlink"
```

## Run

```console
$ findopera organize ./library --write
$ rm './named/Billy Budd'
$ write './named/Billy Budd/something-elses.flac' 'a real folder, not a link'
$ findopera organize ./library --write
$ show './named/Billy Budd/something-elses.flac'
```
