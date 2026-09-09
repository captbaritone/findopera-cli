# One recording named twice in one folder

Two markers for the same recording in the same folder are one folder, listed
once. Naming it twice is not a way to get two of it — and it happens easily,
since the site's suggested filename and a hand-made one both carry the id.

The walk itself goes as deep as the library does; nothing here is at the top.

## Recording 332

```json
{"opera": {"title": "Don Giovanni"}}
```

## Recording 10655

```json
{"opera": {"title": "Sosarme"}}
```

## Library

```tree
twice/findopera-332 flac.txt
twice/Don Giovanni [findopera-332] flac.txt
a/b/c/d/e/findopera-10655.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}[ ({{variant}})]'
```
