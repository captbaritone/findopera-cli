# `findopera --help`

<!-- From 00-findopera.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
Organize a library of opera recordings, using metadata from findopera.com

Usage: findopera <COMMAND>

Commands:
  organize  Work out what each folder should be called, and optionally build it
  get       Show one record whole
  create    Add a record
  edit      Change a record
  link      Attach a UPC to a recording
  unlink    Take a UPC off a recording
  delete    Remove a record
  merge     Fold one record into another that turns out to be the same thing
  describe  List the types, or say what one holds
  search    Look up an id by name
  annotate  Write a recording's notes into a folder
  template  The template language: its syntax, and every field
  graphql   Send a GraphQL query to findopera.com and print the response
  schema    Print the GraphQL schema, or one type from it
  feedback  Send a note to whoever maintains findopera.com
  login     Store a token, so that requests say who is making them
  logout    Forget the stored token
  self      This program itself: whether it is the current version
  init      Write a starter findopera.toml, explaining every setting
  help      Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version

Each folder holds a text file of notes about the recording in it — cast,
conductor, year, orchestra, and where it came from — kept so that the folder
says what it contains without anything having to be opened. `findopera
annotate` writes one:

  cd '~/Music/Sosarme' && findopera annotate 10655

The id in its name is also how this program recognises the folder later, which
is what everything else here is built on.

Start with `findopera init`, then `findopera organize --help`.

`findopera self update` says whether a newer one has been released.

Exit codes:
  0  nothing to report
  1  a recording is missing, a name is not a usable path, two folders want
     the same name, or something was in the way of building
  2  the settings, the template or the arguments are wrong
  3  the API was unreachable, or refused

Results go to stdout; everything else to stderr.
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
