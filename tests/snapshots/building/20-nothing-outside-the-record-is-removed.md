# Nothing outside the record is ever removed

The whole safety argument. A copied folder is indistinguishable from one
somebody made by hand, so this cannot depend on telling them apart — only on
what was written down when it was built.

The second run asks for a different name, which orphans everything the first
one made. The folder it built goes. The folder somebody else put there,
which was never recorded, stays — along with what is in it.

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
$ write './named/My Own Mixes/track.flac' 'nobody asked findopera about this'
$ findopera organize ./library --write -t '{{opera.title}} {{id}}'
$ show './named/My Own Mixes/track.flac'
```
