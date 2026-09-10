# The settings `init` writes

The starter file is the first thing anybody has, and a default that does not
parse would be found by whoever least wants to find it. So a case writes one
and runs against it, with nothing edited.

That covers the template it suggests, the list it now asks for, and — since
`[index]` is a heading and everything below one belongs to it — that the file
is still in an order TOML reads the way it looks.

## Recording 332

```json
{"opera": {"title": "Don Giovanni", "composer": {"lastName": "Mozart", "firstName": "Wolfgang Amadeus"}}, "year": 1959}
```

## Library

```tree
Mozart/findopera-332.txt
```

## Run

```console
$ findopera init ./library
$ findopera organize ./library --tabs
```
