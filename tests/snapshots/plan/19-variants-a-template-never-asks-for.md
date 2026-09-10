# Variants a template never asks for

Both markers declare a variant, and different ones — `flac` and `mp3`. They
still take the same name, because the template never asks what they declare.

So the fix is in the template, not in the markers, and the answer has to say
so: renaming either file would change nothing at all.

## Recording 332

```json
{"opera": {"title": "Don Giovanni"}}
```

## Library

```tree
rips/a/findopera-332 flac.txt
rips/b/findopera-332 mp3.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\]'
```
