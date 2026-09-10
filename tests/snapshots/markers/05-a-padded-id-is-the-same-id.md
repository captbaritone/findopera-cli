# A padded id is the same id

`findopera-075` and `findopera-75` name one recording, not two. Leading zeros
come from whatever wrote the filename and say nothing about which record is
meant.

The proof is the complaint: both folders want the same name, and the reason
given is that the template never asks for `{{variant}}` — which is what two
folders holding *the same* recording look like. Two different recordings
would have rendered two different names without being asked anything.

## Recording 75

```json
{"opera": {"title": "Billy Budd"}}
```

## Library

```tree
padded/findopera-075.txt
plain/findopera-75.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}'
```
