# A box set is listed once per recording

One directory holding two markers stands for both recordings.

## Library

```tree
Box Sets/Donizetti box/findopera-1721.txt
Box Sets/Donizetti box/findopera-5000.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{composer.lastName}}/{{opera.title}}'
```
