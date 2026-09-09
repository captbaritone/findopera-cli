# A symlink follows the folder, not the files in it

The mirroring modes copy or link each file as it stood when they ran. A
symlink points at the folder itself, so what is in it later is what you get —
add a track to the library and it is there through the destination without
running anything again.

That is the reason to choose it, and the reason not to: the destination is
not a second copy of anything, and it goes hollow if the library moves.

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
BillyBudd/track01.flac
```

## Config

```toml
template = "{{opera.title}}"
link = "symlink"
```

## Run

```console
$ findopera organize ./library --write
$ write ./library/BillyBudd/track02.flac 'a track added afterwards'
$ show './named/Billy Budd/track02.flac'
$ rm ./library/BillyBudd/track02.flac
$ show './named/Billy Budd/track02.flac'
```
