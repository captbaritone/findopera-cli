# Merging one singer into another

The first id loses; the one after `--into` survives. Getting them the wrong
way round would merge the survivor into the duplicate, and nothing
downstream could tell. The `Requests` section below is what makes that
visible: `id` is the loser, `intoId` the survivor.

## Answer MergeSinger

```json
{"data":{"mergeSinger":{"id":"456"}}}
```

## Run

```console
$ findopera merge singer 133 --into 456 -m 'https://en.wikipedia.org/wiki/Maria_Callas — one person, two spellings' --yes
```
