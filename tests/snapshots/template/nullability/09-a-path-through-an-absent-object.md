# A path through an object that is not there

`opera.language` is an object, and it may be missing entirely. Reaching
through it for a name is then absence rather than an error — the group drops
as it would for any other field that is not there.

One recording has a language, one has none.

## Recording 1

```json
{"opera": {"title": "Salome", "language": {"name": "German"}}}
```

## Recording 2

```json
{"opera": {"title": "Sosarme", "language": null}}
```

## Library

```tree
german/findopera-1.txt
none/findopera-2.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}[ ({{opera.language}})]'
```
