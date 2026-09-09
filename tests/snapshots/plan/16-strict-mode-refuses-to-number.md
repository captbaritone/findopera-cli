# Strict mode refuses to number

The same tree as 13. Numbering is the designed fallback, so by default it is a
note and the plan can be acted on; under `--require-variants` the caller has
said every duplicate must be named, so each one is spelled out.

## Library

```tree
rips/a/findopera-332.txt
rips/b/findopera-332.txt
elsewhere/findopera-75.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\][ ({{variant}})]' --require-variants
```
