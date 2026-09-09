# A derived number steps over a declared variant

`a` declares the variant `2`, so it is not in the clash — it renders
differently. But numbering its neighbours 1 and 2 would walk straight into it,
so the numbering counts every name not being renumbered as taken and skips to
3.

## Library

```tree
a/findopera-332 2.txt
b/findopera-332.txt
c/findopera-332.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\][ ({{variant}})]'
```
