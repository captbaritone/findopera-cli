# A template without variant cannot be numbered

## Library

```tree
rips/a/findopera-332.txt
rips/b/findopera-332.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\]'
```
