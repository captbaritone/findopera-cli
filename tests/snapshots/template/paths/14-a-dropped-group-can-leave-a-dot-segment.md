# A dropped group can leave a dot segment

A literal `.` segment is caught at parse time, but one that only appears
because a group dropped cannot be — which is why the check survives in
`to_path` as well.

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
    "title": "Salome",
    "englishTitle": null,
    "librettist": null,
    "url": "https://findopera.com/opera/1",
    "language": null,
    "composer": {
      "fullName": "«composer»",
      "firstName": "«composer-first»",
      "lastName": "Strauss",
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
$ findopera organize ./library --tabs -t '{{composer.lastName}}/[{{year}}]./{{opera.title}}'
```
