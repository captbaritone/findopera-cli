# An entry that stopped looking like ours is left alone

Recorded as a folder that was built by copying, and now a file. Somebody did
that, and whatever they meant by it, the record no longer describes what is
there — so it is not ours to remove any more.

It is reported rather than passed over in silence, because a destination
holding something unexplained is worth hearing about.

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
link = "copy"
```

## Run

```console
$ findopera organize ./library --write
$ rm './named/Billy Budd'
$ write './named/Billy Budd' 'a file now, whatever it was before'
$ findopera organize ./library --write -t '{{opera.title}} {{id}}'
$ show './named/Billy Budd'
```
