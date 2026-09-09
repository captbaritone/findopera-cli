# A folder inside another recordings folder is refused

A separator inside an optional group renders different depths, so a recording
that drops the group encloses one that does not. The names differ, so this is
not a clash — but the two cannot both be built, and building them anyway wrote
the second *through* the first into the library being read.

## Library

```tree
rips/plain/findopera-332.txt
rips/flac/findopera-332 flac.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}[/{{variant}}]'
```
