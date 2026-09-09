# One absent placeholder drops a group containing several

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
    "lastName": "Solti",
    "born": null,
    "died": null
  },
  "notedSingers": [],
  "upcs": [],
  "opera": {
    "title": "Salome",
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
$ findopera organize ./library --tabs -t '{{opera.title}}[ - {{conductor.lastName}}, {{orchestra}}]'
```
