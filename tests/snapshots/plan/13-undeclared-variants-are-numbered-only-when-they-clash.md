# Undeclared variants are numbered only when they clash

## Library

```tree
rips/a/findopera-332.txt
rips/b/findopera-332.txt
elsewhere/findopera-75.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\][ ({{variant}})]'
```
