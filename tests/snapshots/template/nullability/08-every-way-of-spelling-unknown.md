# Every way of spelling unknown is absence

findopera.com says a thing is unknown in three ways, depending on the field
and how old the entry is: a zero, a null, and an empty string. All three have
to arrive as absence, or a group holding one of them would render `()` or
` - ` around nothing.

One recording for each spelling. The first two drop the month and keep
nothing else; the third keeps its month — so the template is plainly working
— and drops the orchestra that is an empty string.

## Recording 1

```json
{"opera": {"title": "Zero"}, "month": 0, "orchestra": null}
```

## Recording 2

```json
{"opera": {"title": "Null"}, "month": null, "orchestra": null}
```

## Recording 3

```json
{"opera": {"title": "Empty"}, "month": 4, "orchestra": ""}
```

## Library

```tree
zero/findopera-1.txt
null/findopera-2.txt
empty/findopera-3.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}[ ({{month}})][ - {{orchestra}}]'
```
