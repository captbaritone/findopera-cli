# The same template with the whole series present

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
  "notedSingers": [
    {
      "fullName": "Nilsson",
      "firstName": "«s»",
      "lastName": "«s-last»",
      "born": null,
      "died": null
    },
    {
      "fullName": "Wächter",
      "firstName": "«s»",
      "lastName": "«s-last»",
      "born": null,
      "died": null
    },
    {
      "fullName": "Stolze",
      "firstName": "«s»",
      "lastName": "«s-last»",
      "born": null,
      "died": null
    }
  ],
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
$ findopera organize ./library --tabs -t '{{opera.title}}[ - {{singer1}}][, {{singer2}}][, {{singer3}}]'
```
