# `findopera organize --help`

<!-- From 01-organize.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
Walk a library for notes files, work out what each recording's folder should
be called, and — with --write — build a tree of those folders at the
destination in the settings file.

NOTES FILES

Each folder holds a text file of notes about its recording — the cast, the
conductor, the year, the orchestra, and a link back to where it came from.
They are meant to be read: a folder with one in it says what it holds without
anything having to be opened, and keeps saying so on a disk that outlives this
program.

  findopera annotate 10655

writes one into the current folder, named as findopera.com names it:

  Sosarme, Re di Media-2026-Angioloni [findopera-10655].txt

That name is also how this program recognises the folder again. It looks for
`findopera-<id>` in it, so the file can be renamed freely as long as that part
survives. A bare `10655.txt` is not enough — a number and a .txt is what a
track listing or a year looks like, and the `findopera-` is what says the
number means a recording.

Only the name is matched on, never the contents, so a folder can be claimed by
hand — `touch 'findopera-10655.txt'` works — but that leaves a file with
nothing in it for anyone to read.

One folder may hold several of these; a box set covering several operas is
listed once for each recording in it. Where two folders hold the *same*
recording — a FLAC rip and an MP3 rip of one performance — nothing in the
recording tells them apart, so put a word after the id to say which is which:

  findopera annotate 10655 --variant flac
  findopera annotate 10655 --variant mp3

A template picks that word up as {{variant}}.

BUILDING

Nothing is written without --write.

The destination is kept matching the plan, so a folder built for a recording
that has since left the library is removed again. Only folders this program
recorded building are ever removed — anything else in there is untouched, and
a destination it has no record of is refused rather than guessed at.

With no destination set the folders are still worked out and shown, which is
what you want while you are still settling on a template.

Usage: findopera organize [OPTIONS] [DIR]

Arguments:
  [DIR]
          Directory to walk
          
          [default: .]

Options:
      --config <FILE>
          Settings file. Defaults to findopera.toml beside DIR

  -t, --template <TEMPLATE>
          Template, overriding the one in the settings file.
          
          `{{field}}` placeholders, `|`-separated fallbacks with a quoted literal last, and
          `[optional groups]` dropped when a placeholder inside them turns out to be absent.

      --write
          Actually build it.
          
          Without this, nothing is written: the command says what it would do and stops. The
          destination lives in the settings file rather than on the command line, so this is the
          only thing that says out loud that a run is going to touch the disk.

      --dry-run
          Say so explicitly: make no changes.
          
          This is what happens anyway. It exists so that a run can state its intention rather than
          rely on the absence of a flag, and so that saying both things at once is an error rather
          than a silent winner.

      --follow-links
          Follow symlinks while walking

      --tabs
          Separate the columns with a tab instead of padding, for piping

      --json
          Print the plan as JSON

      --require-variants
          Fail if any folder had to be numbered.
          
          Two rips of one recording get a number each unless a marker says which is which, and a
          number taken from walk order shifts as the library changes. This insists every one of them
          be named.

      --token <TOKEN>
          Token to identify as. Overrides the environment and the stored one.
          
          Prefer `findopera login`, or the FINDOPERA_TOKEN environment variable: a token on the
          command line is visible to anyone who can list processes, and is kept in the shell's
          history.

      --endpoint <URL>
          GraphQL endpoint
          
          [default: https://findopera.com/api/graphql]

  -h, --help
          Print help (see a summary with '-h')

Examples:
  findopera organize ~/Music
  findopera organize ~/Music --write
  findopera organize ~/Music -t '{{opera.title}}[ ({{year}})]'
  findopera organize ~/Music --config ~/Music/by-conductor.toml
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
