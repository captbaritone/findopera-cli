# An underline covers a whole text run escapes included

The text between the placeholders is one run: escapes resolved, braces and
brackets included. If the lexer emitted it as several tokens instead, the
underline would cover only part of it and the segment check would not see
`..` as an interior piece of a single run.

## Recording 1

```json
{
  "id": 1,
  "url": "https://findopera.com/recording/1",
  "year": null,
  "month": null,
  "day": null,
  "orchestra": null,
  "chorus": null,
  "conductor": {
    "fullName": "«conductor»",
    "firstName": "«conductor-first»",
    "lastName": "«conductor-last»",
    "born": null,
    "died": null
  },
  "notedSingers": [],
  "upcs": [],
  "opera": {
    "title": "«title»",
    "englishTitle": null,
    "librettist": null,
    "url": "https://findopera.com/opera/1",
    "language": null,
    "composer": {
      "fullName": "«composer»",
      "firstName": "«composer-first»",
      "lastName": "«composer-last»",
      "born": null,
      "died": null
    }
  }
}
```

## Library

```tree
one/findopera-1.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}/\[a\]/../{{composer.lastName}}'
```
