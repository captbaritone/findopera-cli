# A link left by an earlier run is not a way into the library

The danger is specific. Every path here is followed, so a link sitting in the
destination redirects a build into whatever it points at — and an earlier run
under a different template may have left one pointing back at the library.
Planning refuses folders that nest within a single run, but a link left by a
previous one is invisible to it.

So the build is refused where it meets the link, and the library keeps
exactly what it had. The last two steps are the proof: the folder the link
points into still holds its two files and nothing else.

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
template = "{{opera.title}}/rip"
link = "symlink"
```

## Run

```console
$ findopera organize ./library --write
$ rm './named/Billy Budd'
$ link ./library/BillyBudd './named/Billy Budd'
$ findopera organize ./library --write
$ show ./library/BillyBudd/findopera-75.txt
$ show ./library/BillyBudd/rip
```
