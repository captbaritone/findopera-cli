# A hard link is a second name for the same file

Which is not visible in a listing — a hard link and a copy are both just
files — so this asks the question that tells them apart. Write through the
one in the destination, and read the one in the library.

It comes back changed, because there is only one file. That is what choosing
`hardlink` means, and why it costs no space.

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
link = "hardlink"
```

## Run

```console
$ findopera organize ./library --write
$ append './named/Billy Budd/track01.flac' ' — written through the destination'
$ show ./library/BillyBudd/track01.flac
```
