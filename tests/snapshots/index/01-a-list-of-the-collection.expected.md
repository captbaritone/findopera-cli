# A list of the collection

<!-- From 01-a-list-of-the-collection.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
+ ./library/Britten     Britten/Billy Budd
+ ./library/Mozart      Mozart/Don Giovanni (flac)
+ ./library/Mozart-mp3  Mozart/Don Giovanni (mp3)
+ ./library/Odon        Ödön/Maria Egiziaca
```

## stderr

```
findopera: building a copy of every file in ./named
findopera: listed them in ./named/00 - What is in here.txt
findopera: 4 built, 0 already there, 0 left alone
```

## Destination

```tree
00 - What is in here.txt
Britten/
Britten/Billy Budd/
Britten/Billy Budd/findopera-75.txt
Mozart/
Mozart/Don Giovanni (flac)/
Mozart/Don Giovanni (flac)/findopera-332 flac.txt
Mozart/Don Giovanni (mp3)/
Mozart/Don Giovanni (mp3)/findopera-332 mp3.txt
Ödön/
Ödön/Maria Egiziaca/
Ödön/Maria Egiziaca/findopera-5000.txt
```

## Index

```text title="./named/00 - What is in here.txt"
List of the 3 opera recordings in this collection, as of <date>.

Composer, first - Opera (year.month) Conductor [singers] [rips] (findopera id) — folder

Britten, Benjamin - Billy Budd (1967) Britten [Pears] (findopera 75) — Britten/Billy Budd
Mozart, Wolfgang Amadeus - Don Giovanni (1959.07) Krips [Siepi, della Casa] [flac, mp3] (findopera 332) — Mozart/Don Giovanni (flac)
Ödön, Anton - Maria Egiziaca Ödön (findopera 5000) — Ödön/Maria Egiziaca
```

## Requests

```
Recordings
  {"ids":["75","332","332","5000"]}
```

## Exit

```
0
```
