# Each type merges through its own mutation

One mutation per type rather than a general one, so a table that drifted
would send `mergeSinger` for an opera and be refused in terms naming
neither. The request below says which was sent.

## Answer Merge

```json
{"data":{"mergeOpera":{"id":"34"}}}
```

## Run

```console
$ findopera merge opera 12 --into 34 -m 'https://... — one work, catalogued twice' --yes
```
