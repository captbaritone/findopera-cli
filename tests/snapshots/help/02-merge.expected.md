# `findopera merge --help`

<!-- From 02-merge.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
Merge a record into another, in favour of the second, for when two of them
turn out to describe the same singer, opera or performance.

  findopera merge singer 133 --into 456 -m 'https://... — same person, two spellings'

The first id loses. Its record goes, keeping its history, and anyone arriving
with that id afterwards is sent to the survivor instead, so a link written
down before the merge still works.

A merge is refused while anything still points at the losing record —
recordings against a duplicate singer, say. Move those over first, so that
what became of them is a decision somebody made rather than a side effect. The
refusal comes from the server, and names what is still in the way.

Not every type can be merged. `findopera describe <type>` says whether one
can, and like `delete` this asks for --yes as well as a reason.

Usage: findopera merge [OPTIONS] --into <ID> --message <TEXT> <TYPE> <ID>

Arguments:
  <TYPE>
          What kind of record

  <ID>
          The id that loses, and goes

Options:
      --into <ID>
          The id that survives, and keeps its own fields

  -m, --message <TEXT>
          Source and context for this change, for the record's history

      --yes
          Say so out loud. Nothing is merged without it

      --json
          Print the result as JSON

      --endpoint <URL>
          [default: https://findopera.com/api/graphql]

      --token <TOKEN>
          

  -h, --help
          Print help (see a summary with '-h')

Examples:
  findopera merge singer 133 --into 456 -m 'same person, two spellings' --yes
  findopera merge opera 12 --into 34 -m 'https://...' --yes --json
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
