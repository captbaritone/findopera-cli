# Two distinct recordings that render alike are numbered

332 and 5876 are separate FindOpera recordings of the same performance: same
year, same conductor, same singers. Without the id in the template they are
indistinguishable, and no variant was declared, so they are numbered.

## Library

```tree
a/findopera-332.txt
b/findopera-5876.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{composer.lastName}}/{{opera.title}}[ ({{variant}})]'
```
