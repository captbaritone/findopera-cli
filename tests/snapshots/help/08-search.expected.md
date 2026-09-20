# `findopera search --help`

<!-- From 08-search.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
Find the id of a recording, or of the people and works one is made of.

Everything else here takes an id. This is how you get one.

  findopera search recording tosca --singer callas
  findopera search singer callas
  findopera search opera tosca
  findopera search character scarpia

The first column is the id, so a result can be handed straight to another
command:

  findopera search recording tosca --tabs | head -1 | cut -f1 | xargs findopera annotate

Each kind takes what applies to it. A recording can be narrowed by more than
its title — --singer may be repeated, --year matches within two years either
side, and --upc takes the barcode off the box in whatever form it is printed.
--composer tells apart two works with one title, and --character asks which
recordings have a Scarpia rather than who sang him. The rest take a name and
nothing else, so `findopera search singer --help` is short.

Searching is case and accent insensitive and matches part of a name, so
`boheme` finds `La Bohème`.

Only the first --first matches are shown, 10 by default, and 200 is the most
that can be asked for at once. When there are more it says so, and says which
--offset reaches the next of them — narrowing is still the better answer to a
search, but a full page should never be mistaken for the whole of it, and
something that has to be walked should be walkable.

--offset is on recordings alone. The catalog's other searches take a count and
nothing else, so there is nowhere for it to go on them.

Composers, singers, conductors and characters are what a recording is made of,
and their ids are what it takes to describe a new one.

Usage: findopera search <COMMAND>

Commands:
  recording  A recording, by its opera, composer, cast, conductor, year or barcode
  opera      An opera, by title
  singer     A singer, by name
  conductor  A conductor, by name
  composer   A composer, by name
  character  A character, by name
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

Examples:
  findopera search recording boheme --singer pavarotti --singer freni
  findopera search recording boheme --composer leoncavallo
  findopera search recording tosca --conductor 'de sabata' --year 1953
  findopera search conductor karajan --first 10
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
