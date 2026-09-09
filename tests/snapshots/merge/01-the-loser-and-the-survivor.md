# Merging one singer into another

The first id loses; the one after `--into` survives. Getting them the wrong
way round would merge the survivor into the duplicate, and nothing
downstream could tell. The `Requests` section below is what makes that
visible: `id` is the loser, `intoId` the survivor.

## Answer Merge

```json
{"data":{"mergeSinger":{"id":"456"}}}
```

## Run

```console
$ findopera merge singer 133 --into 456 -m 'same person, two spellings' --yes
```

<!-- Everything below is generated. UPDATE_EXPECT=1 cargo test -->

## stdout

```
456
```

## stderr

```
findopera: singer 133 is now singer 456
```

## Destination

```tree
```

## Requests

```
Merge
  {"id":"133","intoId":"456","justification":"same person, two spellings"}
```

## Exit

```
0
```
