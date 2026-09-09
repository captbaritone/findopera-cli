# A value of only control characters sanitizes away

Sanitizing turns control characters into spaces and then trims, so a value
that is present, non-empty and schema-honest can still leave its segment
with nothing in it.

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
    "title": "\u0007\u0007",
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
$ findopera organize ./library --tabs -t '{{opera.title}}'
```
