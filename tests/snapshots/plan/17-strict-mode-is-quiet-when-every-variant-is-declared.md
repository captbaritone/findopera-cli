# Strict mode is quiet when every variant is declared

Nothing was numbered, so strict mode has nothing to say.

## Library

```tree
rips/flac/findopera-332 flac.txt
rips/mp3/findopera-332 mp3.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\][ ({{variant}})]' --require-variants
```
