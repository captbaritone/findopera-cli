# `findopera search recording --help`

<!-- From 04-search-recording.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
A recording, by its opera, cast, conductor, year or barcode

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
