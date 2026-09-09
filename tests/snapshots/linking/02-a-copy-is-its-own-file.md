# A copy is a file of its own

The same question, asked of `copy`, and the answer is the other one. Writing
through the destination leaves the library exactly as it was, because there
are two files now and they only started out alike.

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
$ append './named/Billy Budd/track01.flac' ' — written through the destination'
$ show ./library/BillyBudd/track01.flac
$ show './named/Billy Budd/track01.flac'
```
