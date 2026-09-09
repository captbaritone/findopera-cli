# What counts as a marker

One library holding every near-miss at once, so which files are recognised is
a list rather than an argument. Only the four with `findopera-<id>` in the
name reach the listing; the rest are what a library is full of.

- a bare number and a `.txt` is a track listing, a year, a disc number;
- `notfindopera-75` merely ends in the token;
- `findopera` with no number says nothing about which recording;
- an image is not notes, whatever it is called.

The token may sit anywhere in the name, in whatever brackets the file
happened to arrive with, which is why the site's own suggested filename works
untouched.

## Recording 75

```json
{"opera": {"title": "Billy Budd"}}
```

## Recording 1721

```json
{"opera": {"title": "Aida"}}
```

## Recording 5000

```json
{"opera": {"title": "Maria Egiziaca"}}
```

## Recording 10655

```json
{"opera": {"title": "Sosarme"}}
```

## Library

```tree
yes-plain/findopera-75.txt
yes-suggested/Sosarme, Re di Media-2026 [findopera-10655].txt
yes-parens/Salome [Live] (findopera-1721).txt
yes-dotted/Aida.findopera-5000.txt
no-bare-number/10655.txt
no-year/1967.txt
no-disc/01.txt
no-suffix/notfindopera-75.txt
no-notes/liner notes.txt
no-id/Sosarme [findopera].txt
no-image/findopera-10655.jpg
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}'
```
