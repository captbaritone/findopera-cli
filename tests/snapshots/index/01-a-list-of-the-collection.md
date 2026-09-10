# A list of the collection

The tree is filed by composer, so nothing in it says who sang or what year —
and finding a Tosca in it means already knowing that Tosca is Puccini's. The
index is the one file where the whole collection can be read at once, which
is what somebody browsing over a share actually needs.

Note the Don Giovanni. Two rips of it are two folders in the tree, because
they are two things on a disk, but one line in the list naming both — to
somebody asking whether you have it, they are one recording.

## Recording 332

```json
{"opera": {"title": "Don Giovanni", "composer": {"lastName": "Mozart", "firstName": "Wolfgang Amadeus"}},
 "year": 1959, "month": 7, "day": null,
 "conductor": {"fullName": "Josef Krips", "firstName": "Josef", "lastName": "Krips", "born": null, "died": null},
 "notedSingers": [{"fullName": "Cesare Siepi", "firstName": "Cesare", "lastName": "Siepi", "born": null, "died": null},
                  {"fullName": "Lisa della Casa", "firstName": "Lisa", "lastName": "della Casa", "born": null, "died": null}]}
```

## Recording 75

```json
{"opera": {"title": "Billy Budd", "composer": {"lastName": "Britten", "firstName": "Benjamin"}},
 "year": 1967, "month": null, "day": null,
 "conductor": {"fullName": "Benjamin Britten", "firstName": "Benjamin", "lastName": "Britten", "born": null, "died": null},
 "notedSingers": [{"fullName": "Peter Pears", "firstName": "Peter", "lastName": "Pears", "born": null, "died": null}]}
```

## Recording 5000

```json
{"opera": {"title": "Maria Egiziaca", "composer": {"lastName": "Ödön", "firstName": "Anton"}},
 "year": null, "month": null, "day": null,
 "conductor": {"fullName": "Anton Ödön", "firstName": "Anton", "lastName": "Ödön", "born": null, "died": null},
 "notedSingers": []}
```

## Library

```tree
Mozart/findopera-332 flac.txt
Mozart-mp3/findopera-332 mp3.txt
Britten/findopera-75.txt
Odon/findopera-5000.txt
```

## Toml

```toml
template = '''{{composer.lastName}}/{{opera.title}}[ ({{variant}})]'''
destination = "{destination}"
link = "copy"

[index]
file = "00 - What is in here.txt"
header = '''
List of the {{count}} opera recordings in this collection, as of {{date}}.

Composer, first - Opera (year.month) Conductor \[singers\] \[rips\] (findopera id) — folder
'''
template = '''{{composer.lastName}}, {{composer.firstName}} - {{opera.title}}[ ({{year}}[.{{month}}])] {{conductor.lastName}}[ \[{{singers.lastNames}}\]][ \[{{variants}}\]] (findopera {{id}}) — {{path}}'''
```

## Run

```console
$ findopera organize ./library --write
```
