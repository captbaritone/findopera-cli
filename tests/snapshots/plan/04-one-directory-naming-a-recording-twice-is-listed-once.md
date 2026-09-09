# One directory naming a recording twice is listed once

Two markers, one directory, one recording, neither saying anything about a
rip — a marker and a renamed copy of it. The directory is named once.

## Library

```tree
rips/only/findopera-332.txt
rips/only/Don Giovanni [findopera-332].txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}} \[{{id}}\]'
```
