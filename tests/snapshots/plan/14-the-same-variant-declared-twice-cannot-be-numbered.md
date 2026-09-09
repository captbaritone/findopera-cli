# The same variant declared twice cannot be numbered

The markers say the same thing, so the template is not at fault and the
message must not send the reader to it.

## Library

```tree
rips/a/findopera-332 flac.txt
rips/b/findopera-332 flac.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\][ ({{variant}})]'
```
