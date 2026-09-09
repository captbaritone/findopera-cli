# Fields projected out of a list

A cast is a list, and the fields built from it are the useful shapes of it:
the first singer, all of them joined, their surnames only. A list the schema
guarantees is present can still be empty, so each of those can be absent —
which is why the second recording renders none of them.

## Recording 1

```json
{"opera": {"title": "Tosca"}, "notedSingers": [{"fullName": "Maria Callas", "firstName": "Maria", "lastName": "Callas", "born": null, "died": null},{"fullName": "Giuseppe di Stefano", "firstName": "Giuseppe", "lastName": "di Stefano", "born": null, "died": null}]}
```

## Recording 2

```json
{"opera": {"title": "Sosarme"}, "notedSingers": []}
```

## Library

```tree
cast/findopera-1.txt
none/findopera-2.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}[ - {{singer1}}][ ({{singers.lastNames}})]'
```
