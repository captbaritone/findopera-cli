# `findopera search recording --help`

<!-- From 04-search-recording.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
A recording, by its opera, composer, cast, conductor, year or barcode

Usage: findopera search recording [OPTIONS] [QUERY]...

Arguments:
  [QUERY]...
          The name, or part of one
          
          [default: ""]

Options:
      --first <N>
          How many results, up to 200
          
          [default: 10]

      --tabs
          Separate the columns with a tab instead of padding, for piping

      --json
          Print the results as JSON

      --endpoint <URL>
          GraphQL endpoint
          
          [default: https://findopera.com/api/graphql]

      --token <TOKEN>
          Token to identify as

      --singer <NAME>
          A singer on the recording. May be repeated; all must appear

      --conductor <NAME>
          The conductor of the recording

      --composer <NAME>
          The composer of the opera.
          
          Two works can share a title -- `La Boheme` is Puccini's and Leoncavallo's -- so this is
          what tells them apart.

      --character <NAME>
          A role somebody sings on it.
          
          About the work rather than the cast: which recordings have a Scarpia, whoever sang him.

      --in-collection <ID>
          Only what is in this list of yours.
          
          By the list's id, which `findopera collection list` gives. Needs a token, since a list
          belongs to an account.

      --local
          Only what this library holds.
          
          Reads `[collection]` from the findopera.toml beside you and asks for that list, so what
          counts as held is whatever `organize --write` last put there. Nothing on the disk is read.

      --offset <N>
          How many to step over first, for walking past a full page.
          
          Recordings only: the catalog's other searches take a count and nothing else, so there is
          nowhere for this to go on them.
          
          [default: 0]

      --config <FILE>
          Which settings file --local should read.
          
          Nothing searches up the tree, so without this it is the findopera.toml in the current
          folder.

      --year <YEAR>
          Recorded within two years of this

      --upc <CODE>
          The barcode off the box.
          
          However it is written: spaces and dashes are ignored, and the 12, 13 and 14 digit forms of
          the same code all find each other.

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
