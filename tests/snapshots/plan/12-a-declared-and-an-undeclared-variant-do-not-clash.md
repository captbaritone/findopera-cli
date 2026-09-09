# A declared and an undeclared variant do not clash

Only one of these says what it is, and that is enough: the two render
differently, so there is nothing to number.

## Library

```tree
rips/flac/findopera-332 flac.txt
rips/other/findopera-332.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\][ ({{variant}})]'
```
