# The id separates two distinct recordings

## Library

```tree
a/findopera-332.txt
b/findopera-5876.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{composer.lastName}}/{{opera.title}} \[{{id}}\]'
```
