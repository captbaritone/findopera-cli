# `findopera delete --help`

<!-- From 06-delete.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
Remove a record.

Nothing here is really destroyed — every change is versioned and can be
reverted — but this takes something away, so it asks for --yes as well as a
reason. `findopera merge` is the other one that does, and asks the same.

Where a record is going because something else says the same thing, prefer
merge: it leaves the old id pointing at the survivor, and this does not.

Usage: findopera delete [OPTIONS] --message <TEXT> <TYPE> <ID>

Arguments:
  <TYPE>
          What kind of record

  <ID>
          Its id

Options:
  -m, --message <TEXT>
          Source and context for this change, for the record's history

      --yes
          Say so out loud. Nothing is removed without it

      --json
          Print the result as JSON

      --endpoint <URL>
          [default: https://findopera.com/api/graphql]

      --token <TOKEN>
          

  -h, --help
          Print help (see a summary with '-h')
```

## stderr

```
```

## Destination

```tree
```

## Requests

```
```

## Exit

```
0
```
