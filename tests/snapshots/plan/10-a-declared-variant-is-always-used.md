# A declared variant is always used

Nothing clashes here, and the variant is still honoured: a word the marker
carries is a statement about the rip, not a tiebreaker to be used only when
one is needed.

## Library

```tree
rips/flac/findopera-332 flac.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\][ ({{variant}})]'
```
