# A group drops around a value that is not there

The whole point of `[…]`: it goes entirely when a placeholder inside it turns
out to be absent, taking its separator with it. So the same template gives a
name with a year and a name without one, and neither has a stray bracket or
a hanging space.

Two recordings, said only by how they differ from a real one — one has a
year, one does not.

## Recording 1

```json
{"opera": {"title": "Salome"}, "year": 1905}
```

## Recording 2

```json
{"opera": {"title": "Elektra"}, "year": null}
```

## Library

```tree
Salome/findopera-1.txt
Elektra/findopera-2.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}[ ({{year}})]'
```
