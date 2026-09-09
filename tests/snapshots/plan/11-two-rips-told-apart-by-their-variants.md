# Two rips told apart by their variants

## Library

```tree
rips/flac/findopera-332 flac.txt
rips/mp3/findopera-332 mp3.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\][ ({{variant}})]'
```
