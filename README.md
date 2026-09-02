# findopera

Name a music library from FindOpera metadata, through a template.

## Installing

```bash
# macOS and Linux
curl --proto '=https' --tlsv1.2 -LsSf \
  https://github.com/captbaritone/findopera-cli/releases/latest/download/findopera-installer.sh | sh
```

```powershell
# Windows
irm https://github.com/captbaritone/findopera-cli/releases/latest/download/findopera-installer.ps1 | iex
```

Or take a binary from [the releases page](https://github.com/captbaritone/findopera-cli/releases)
and put it on your PATH. Builds are published for macOS (Apple silicon and
Intel), Linux (x86-64 and ARM) and Windows.

The Linux builds are statically linked against musl rather than glibc, so they
run on a NAS as well as on a desktop — which is where a music library usually
lives.

With a Rust toolchain, `cargo install --git https://github.com/captbaritone/findopera-cli`
works too.

## Getting started

```bash
findopera search recording tosca --singer callas
findopera annotate 10655   # write a recording's notes into this folder
findopera init ~/Music     # write a findopera.toml, every setting explained
findopera organize ~/Music # see what each folder would be called
```

`findopera.toml` sits beside the library and holds the settings that should not
change between runs — the template above all, since a library named by two
slightly different templates is worse than one named by either:

```toml
# The name to give each folder. `/` separates folder levels.
template = '''
{{composer.lastName}}[ ({{composer.dates}})]/{{opera.title}}[ ({{year}})]
'''

require-variants = false
follow-links = false
```

The template goes in a `'''` block, which is the only TOML form where what you
write is exactly what you would type at the prompt: a plain `'…'` string cannot
hold the apostrophe in `{{opera.title|"L'…"}}`, and a `"…"` string needs every
`\[` written `\\[`.

Nothing searches up the tree. `organize` reads the file beside what you scanned, or
the one you name with `--config` — so one library can carry several, one per
way of naming it, and which one ran is never a guess:

```bash
findopera organize ~/Music --config ~/Music/by-conductor.toml
findopera organize ~/Music -t '{{opera.title}}'  # or just override the template
```

## Reading the library

`findopera organize` walks a directory for notes files and shows what each
folder would be called:

```bash
$ findopera organize '{{composer.lastName}}/{{opera.title}}[ ({{year}})]' ~/Music
./Box Sets/Donizetti box      Donizetti/L'elisir d'amore (1969)
./Box Sets/Donizetti box      Respighi/Maria Egiziaca (1980)
./Britten/Billy Budd (Decca)  Britten/Billy Budd (1967)
./Handel - Sosarme 2026       Handel/Sosarme, Re di Media (2026)
```

## Reading and changing records

Every type in the database can be read, added to, changed and removed:

```bash
$ findopera get recording 264
$ findopera get singer 133 --json

$ findopera describe singer          # what a create takes
$ findopera describe singer --json   # the same, as a JSON Schema

$ echo '{"firstName":"Maria","lastName":"Callas"}' \
    | findopera create singer -m 'https://en.wikipedia.org/wiki/Maria_Callas'
$ echo '{"died":1977}' | findopera edit singer 133 -m 'https://...'
$ findopera delete singer 133 -m '...' --yes
```

`findopera describe` lists the twenty types. Input is JSON on stdin or from a
file, and the fields are checked here before anything is sent — a misspelling
is answered in the terms you used rather than as a GraphQL error about an input
type you never mentioned.

Every change needs `-m`: a source, ideally a URL, and enough context for
someone reading the history later to judge it. The server requires one.

The queries live in [`schema/get.graphql`](schema/get.graphql), curated per
type and validated against the schema by codegen. The table of types and their
input fields is generated from the schema, so twenty types cost the same to
keep current as one.

### Creating a recording whole

A recording, its cast, its source and its barcode go in one request, so a
failure leaves nothing half-made:

```bash
$ findopera create recording -m 'https://example.com/liner-notes' <<'JSON'
{ "operaId": "88", "conductorId": "106", "year": 1953,
  "sourceUrl": "https://example.com/liner-notes",
  "upc": "0012345678998",
  "portrayalInputs": [
    {"singerId": "133", "characterId": "908", "noted": true},
    {"singerId": "142", "characterId": "909", "noted": true},
    {"singerId": "151", "characterId": "910", "noted": true},
    {"singerId": "160", "characterId": "911", "noted": false}
  ] }
JSON
```

`describe recording` lists those keys alongside the ordinary fields. Leave them
all out and it is an ordinary create — the same mutation with nothing extra in
it, so there is no mode to be in.

Exactly three roles take top billing, or all of them when a cast is smaller
than three. That is a rule of this one call rather than a fact about
recordings, and it is checked before a twelve-role payload is sent.

`noted` is required on a portrayal given to a create, though it is optional on
one created by itself. `describe recording` shows the shape of a portrayal
under the field, and `--json` gives it an `items` subschema, so a validator can
check the cast rather than only that it is a list.

### Attaching a barcode

A recording and a UPC are joined rather than owned, so they have their own
verbs:

```bash
$ findopera create upc <<< '{"upc":"0013491103020"}' -m 'back of the box'
$ findopera link recording 264 --upc '0013-491 103020' -m 'Decca reissue'
$ findopera unlink recording 264 --upc 0013491103020 -m 'wrong release'
```

The UPC has to exist first — nothing conjures one from a typo. Punctuation in
the code is dropped, but unlike `search --upc` the digits are not padded: this
names a record to attach, and guessing at which is not the same as finding one.

### Errors

One convention everywhere. A person gets the server's words on stderr:

```
findopera: the server refused the request:
    [NOT_FOUND] there is no singer with the id 999999999
```

With `--json`, the same facts in GraphQL's own shape — `message`, `code`,
`path` — also on stderr, since stdout carries results and a failure has none.
A `path` is what makes an error about one field rather than the whole request.
The exit status says which happened either way.

## Finding an id

Everything here takes an id, so `search` is how you get one:

```bash
$ findopera search recording tosca --conductor 'de sabata'
264  Tosca  1953  Sabata  Callas, di Stefano, Gobbi

$ findopera search singer callas
133  Maria Callas  1923–1977
```

The kinds are `recording`, `opera`, `singer`, `conductor`, `composer` and
`character`, and each is its own subcommand, so `findopera search singer
--help` offers only what applies to a singer. Matching is partial and ignores
case and accents, so `boheme` finds `La Bohème`.

A recording can be narrowed by more than its title — `--singer` may be
repeated, `--year` matches within two years either side, and `--upc` takes the
barcode off the box:

```bash
$ findopera search recording --upc 028941742827
75  Billy Budd  1967  Britten  Pears, Shirley-Quirk, Tear
```

The barcode is matched however it is written. Spaces and dashes are ignored,
and the same product is stored here as 12, 13 or 14 digits depending on where
the code came from — all three forms find each other, so whichever is printed
on the box works. The id is the
first column, so a result can be handed straight to the next command:

```bash
findopera search recording tosca --tabs | head -1 | cut -f1 | xargs findopera annotate
```

A search shows the first `--first` matches, 10 by default, and says so when
there are more:

```
findopera: these are the first 3, and there are more. Narrow the search, or raise --first.
```

At most 200 can be asked for at once; the server refuses more, and so does the
CLI, before spending the request. There is no way to page through them, on
purpose — narrowing is a better
answer than paging, and a result you had to walk to is one you could have asked
for. What matters is that a full page is never mistaken for the whole answer:
something that matched perfectly well would look like it does not exist.
`--json` carries the same fact as `truncated`, beside `results`.

The other kinds exist because a recording is made of them: describing a new one
through the API means naming its opera, its conductor, and a singer and
character for every role.

The queries live in [`schema/search.graphql`](schema/search.graphql) and are
validated against the schema by codegen, so a field renamed upstream fails the
build rather than someone's search. Codegen cannot catch an operation renamed
in that file — the document stays valid — so a test checks that seam
separately.

## Notes files

Each folder holds a text file of notes about its recording, written by
`findopera annotate`:

```bash
$ cd '~/Music/Handel - Sosarme 2026'
$ findopera annotate 10655
./Sosarme, Re di Media-2026-Angioloni [findopera-10655].txt
```

```
                    SOSARME, RE DI MEDIA

           by George Frideric Handel (1685-1759)

                         -= Cast =-
    Sosarme ......................... Rémy Brès-Feuillet
    Elmira ............................... Sarah Charles
    ...

Conductor: Marco Angioloni
Recorded: 2026
Orchestra: Orchestre de l'Opéra Royal

           https://findopera.com/recording/10655
```

They are meant to be read. A folder with one in it says what it holds without
anything having to be opened, and goes on saying so on a disk that outlives
this program and the service behind it.

The name is what `organize` recognises later: it looks for `findopera-<id>`,
where the id is the number in the recording's address. The file can be renamed
freely as long as that part survives.

```
findopera-10655.txt                            fine
Sosarme, Re di Media [findopera-10655].txt     also fine
```

A bare `10655.txt` is deliberately not enough — a number and a `.txt` is what
a track listing, a year or a disc number looks like, and the `findopera-` is
what says the number means a recording.

One folder can hold several: a box set covering several operas is listed once
per recording, as above.

Folders can be skipped, which on a network drive is worth more than it sounds
— every folder looked at is a round trip:

```toml
ignore = ["@eaDir", "Incomplete", "**/Artwork/**"]
```

A pattern is matched against a folder's own name and against its path from the
library root, so `@eaDir` skips one wherever it appears while `Unsorted/**`
skips only that one. A skipped folder is not looked inside at all.

The walk itself is spread across threads, which matters for the same reason.
On an SMB library, 2,167 folders took 47s walked one at a time and 6s walked
in parallel.

Nothing opens the files. Deciding by content would mean reading every `.txt`
in the library to learn that almost none are markers — on a 12,500-file tree
that is 246ms against 66ms, nearly all of it wasted, and far worse over a
network mount where opening a file costs so much more than listing one. It
It also means a folder can be claimed without the network — `touch
findopera-10655.txt` is enough to be matched — though that leaves a file with
nothing in it for anyone to read, which is most of the point of having one.

Results go to stdout, one line per recording; everything else to stderr.
`--tabs` separates the columns with a tab instead of padding, for piping.

## Building the tree

The same command builds it, given a destination and `--write`:

```toml
destination = "/Volumes/Opera/named"
link = "symlink"        # or "hardlink" or "copy"
```

**Nothing is written unless you say `--write`.** On its own, `organize` says
what it would do and stops:

```bash
$ findopera organize ~/Music
findopera: would build a link to each folder in /Volumes/Opera/named
+ /Volumes/Opera/named/Britten/Billy Budd [75]
findopera: 1 to build, 0 already there, 0 left alone
findopera: nothing was written. To build it, run:
    findopera organize ~/Music --write
```

The destination lives in the settings file rather than on the command line, so
`--write` is the only thing that says out loud that a run is about to touch the
disk.

The three ways of getting there are not spellings of one operation — the
system forces them apart:

| | what it makes | a track added later | across disks |
|---|---|---|---|
| `symlink` | one link to the folder | **appears** | yes |
| `hardlink` | every file linked, sharing its contents | no | **no** |
| `copy` | every file copied | no | yes |

**Nothing in your library is ever touched, and nothing in the destination that
this program did not put there.** Anything already in place is left
alone and counted, so running it again after adding one recording does one
thing. Symlinks are written relative to where they sit, so a share reachable by more
than one name — `/volume1/Opera` on a NAS and `/Volumes/Opera` over AFP — reads
correctly from both, and the whole tree can be moved without breaking.

`symlink` cannot merge — a link and a folder cannot share a name, so it
stops and says what is there — while the other two fill a folder that already
exists without writing over anything in it.

What can be known before writing is checked first, because half a tree is
worse than none: a plan with a clash, a destination inside the library it
reads, or a hard link asked to cross a disk:

```
findopera: `link = "hardlink"` cannot reach from /Volumes/Opera/rips to /Users/me/named:
they are on different disks, and a hard link is a second name for a file on the disk
it already lives on. Use `link = "symlink"` to point at it instead, or `link = "copy"`
to have two of it.
```

Nothing is moved or removed, so a renamed source leaves its old entry behind.

## Two rips of one recording

A FLAC rip and an MP3 rip of the same performance are the same *recording*, so
no template separates them from the metadata alone. Only the person who has
both knows what makes them different — and the marker's filename is where they
say so. Anything after the id is a **variant**:

```
findopera-332 flac.txt   findopera-332 mp3.txt   Don Giovanni [findopera-332] SACD.txt
```

which a template picks up as `{{variant}}`. It is nullable, so it wants a
group, and that group vanishes for every recording you only have once:

```bash
$ findopera organize '{{opera.title}} \[{{id}}\][ ({{variant}})]' ~/Music
./rips/flac  Don Giovanni [332] (flac)
./rips/mp3   Don Giovanni [332] (mp3)
./billy      Billy Budd [75]
```

Where no variant was given and two directories still want one name, `organize`
numbers them by walk order. That is the designed fallback, so the plan is
complete and can be acted on — but the numbers are not names: add a third rip
that sorts first and the two already there are renumbered. So it says so, in
one line, and carries on:

```
findopera: 2 directories were numbered by walk order because no variant was
declared; those numbers shift as the library changes. Write a word into each
marker to fix them — mv 'rips/a/findopera-332.txt' 'rips/a/findopera-332 <word>.txt' — or pass
--require-variants to make this an error.
```

`--require-variants` turns that into a failure and spells out every marker,
for a script that means to leave nothing unnamed.

A clash that numbering cannot fix always fails: either the markers declare the
same variant, or the template never mentions `{{variant}}` — and the report
says which, since those send you to different files.

```bash
$ findopera template  # the syntax, and every field with whether it is always there
```

A bad template never costs a network round trip — it is checked against the
schema first:

```bash
$ findopera organize '{{composer.lastName}}/{{year}}' ~/Music
findopera: {{year}} may be absent, and is not inside a group
  {{composer.lastName}}/{{year}}
                        ^^^^^^^^
  help: Add a fallback like {{…|"Unknown"}}, or wrap it in a group so it can be dropped: [{{year}}]
```

| Exit | |
|---|---|
| 0 | nothing to report |
| 1 | a recording is missing, a name is not a usable path, two folders want the same name, or something was in the way of building |
| 2 | the settings, the template or the arguments are wrong |
| 3 | the API was unreachable, or refused |

## Talking to findopera.com

Every request identifies itself — `User-Agent: findopera-cli/0.1.0`, taken
from the package version so it cannot fall behind a release. One function
builds every request, so a new one cannot quietly say nothing.

A response carrying top-level `errors` is fatal, even when data came with it:
a null in a `@semanticNonNull` position is explained by exactly one of those
errors, so partial data cannot be trusted to be whole. The words come through
as written, one to a line, which is what lets the server say something worth
reading to whoever is running an old copy:

```
findopera: the server refused the request:
    [CLIENT_TOO_OLD] findopera-cli 0.1.0 is no longer supported. Upgrade to 0.3 or later
```

### Saying who you are

Requests are anonymous by default, which is enough to read. A token makes them
yours:

```bash
findopera login --new           # findopera.com issues one on the spot
findopera login --new --email you@example.com
findopera login < token.txt     # or paste one you already have
export FINDOPERA_TOKEN=...      # or this, for anything unattended
findopera logout
```

`--new` asks for no account and no proof of who you are. The token is not there
to identify you; it is there so your requests can be told apart from everyone
else's — which is what lets an edit be attributed, a bad run be found together
and undone, and your reads be given a limit of their own. Edits made with it
are recorded under a name the server gives you.

`--email` is optional and never verified. It is somewhere to reach you if
something you are doing turns out to be blocked — a limit, or a bug at the
other end — which is otherwise impossible for an account nobody proved anything
to get. It is not a login, it is not published, and nothing in the schema can
read it back.

findopera.com keeps only a hash of the token, so it is shown once and cannot be
looked up again. A lost one is replaced, not recovered.

The token is *not* kept in `findopera.toml`. That file lives inside the library
being organized — the scan walks it, `--write` can link the folder holding it
into the destination tree, and libraries sit on network shares and in sync
folders. A secret there leaks by construction. It goes in your own
configuration directory instead, created mode 600 rather than written and then
tightened, since between those two there is a moment where it is readable. If
it is ever found readable by anyone else, the run stops and says so rather than
carrying on anonymously.

An argument beats the environment, which beats the stored token, so a one-off
never needs the stored one moved out of the way.

Identity rides on *every* request, not only the ones that change something.
Reading is most of what this program does — a library of three thousand markers
is thirty requests — and a server that cannot tell those from a stranger's has
to treat them like a stranger's. Identifying the reads is what earns them a
limit of their own. `tests/auth.rs` asserts this on the bytes on the wire,
including for the query generated into the binary, which no caller passes in
and is therefore the easiest one to leave anonymous.

### Asking it anything else

`organize` covers the one question this program exists to answer. For the
rest — looking up an id, checking what a recording holds — there is a way
through to the API itself:

```
$ findopera graphql '{ searchOperas(query: "Tosca", first: 2) { id title } }'
{
  "data": {
    "searchOperas": [
      { "id": 88, "title": "Tosca" }
    ]
  }
}
```

The query can be an argument, `--file`, or standard input; the last is the one
to reach for from a script, since a GraphQL document is full of braces and
quotes and often names people. The response goes to stdout as it arrived, so
it can be piped to `jq`. A refusal is still printed — it names the field it
objected to, which is the useful part — but the messages are repeated on
stderr and the exit status is 3, so nothing can mistake one for an answer.

Requests are anonymous, which is enough to read. Mutations need an editor
account and are refused.

To find out what may be asked for, `findopera schema` prints the SDL the
server is currently serving. It is fetched rather than built in, because the
server gains fields between releases of this program. The whole thing is long,
so naming a type prints just that one:

```
$ findopera schema Mutation
$ findopera schema Recording
```

## Using it as a library

The naming is a small template language, usable on its own:

```rust
let tmpl = Template::parse("{{composer.lastName}}/{{opera.title}}[/{{year}}]", FIELDS)?;
let rendered = tmpl.render(&recording);      // "Handel/Sosarme, Re di Media/2026"
to_path(&rendered)?;                         // ["Handel", "Sosarme, Re di Media", "2026"]
```

`render` is total — no `Result`. For any template this crate parsed and any
record, there is a string. Whether that string is a usable relative path is a
separate question, and the only one still decided per record.

## The template language

A placeholder is `{{field}}`. Alternatives separated by `|` are tried left to
right, and a quoted literal serves as a last resort:

```
{{opera.englishTitle|opera.title}}/{{year|"n.d."}} - {{conductor.lastName}}
```

A `[…]` group is dropped whole when a placeholder inside it turns out to be
absent, which is how a separator vanishes along with the value it was
separating — `[ - {{year}}]` contributes nothing at all rather than leaving a
dangling ` - `.

`\[ \] \{ \} \\` are escapes. Whitespace inside a placeholder is
insignificant.

Absent data is never silently dropped, so a missing year can't quietly become
`Handel//2026`. Values are sanitized so a `/` inside a title can't introduce an
unintended path separator, and a rendered result can't be an absolute path or
contain `..`.

## What parsing settles

As much as possible is decided against the schema alone, with no record in
hand, so a bad template fails once rather than on whichever record first
exposes it:

| Template | Rejected at parse time because |
|---|---|
| `{{composer.surname}}` | no such field; the help lists `composer`'s real ones |
| `{{year}}` | `year` may be absent, and there is no group to drop |
| `{{opera.title\|"Untitled"}}` | `opera.title` is never absent, so the literal is dead |
| `[{{id}}]` | `id` is never absent, so the group can never be dropped |
| `[ - [{{year}}]]` | the outer group has no placeholder of its own |
| `/{{composer.lastName}}` | renders an absolute path |
| `{{a}}/../{{b}}` | `..` is not a usable segment |

The middle four need nullability, which is why `FieldDoc` carries it.
`FieldDoc::new` declares a field that may be absent and `FieldDoc::non_null`
one that is always present; nullable is the safe direction to be wrong in,
since claiming a field is always there just moves its failure back to render
time.

That is what leaves `render` with nothing to fail at. A placeholder outside a
group is only accepted when some alternative is always present, so there is no
case where rendering has nothing to write.

## What's left per record

Only whether the rendered string works as a relative path:

| Rendered | `to_path` |
|---|---|
| `""` | `path_empty` — a value sanitized away, or everything droppable dropped |
| `/Salome` | `path_absolute` — a group before the `/` dropped |
| `Strauss/./Salome` | `path_traversal` — a group dropped out of a segment |

Each needs a value to provoke it: the same template renders fine for the next
record. Note the asymmetry with the parse-time path checks — a leading `/`
*written in the template* is caught once, at parse; one that only appears
because a group dropped cannot be, so the check survives in `to_path` too.

## The data seam

The engine knows nothing about any particular record type. It takes two
things, kept apart because they are needed at different times:

- a **schema** — a `&[FieldDoc]` naming every path a template may reference
  and whether it can be absent. Plain data, because parsing has to reason
  about it with no value in hand.
- a **resolver** — a `Fields` impl, with one method per nullability.
  Behavior, because only rendering needs it.

```rust
static FIELDS: &[FieldDoc] = &[
    FieldDoc::new("year", "Year recorded"),                      // may be absent
    FieldDoc::non_null("opera.title", "Title in the original"),  // always there
];

impl Fields for Recording {
    fn required(&self, path: &str) -> String { … }          // non_null fields
    fn optional(&self, path: &str) -> Option<String> { … }  // nullable fields
}
```

The two methods mirror the split the schema declares, and that is what makes
rendering total: a path routed to `required` was declared `non_null`, and
returning a `String` for it is not optional.

For `optional`, `None` means absent — the only distinction the language draws.
An implementation should collapse its own sentinels for unknown (SQL `NULL`,
`0`, `""`) into `None`; better still, collapse them where the record is
deserialized, so the type carries the distinction and this impl has nothing
left to decide.

Neither `Template` nor `parse` is generic over a record type: a parsed
template is just an AST, and a type parameter it never uses would rule out
loading a schema at runtime. The eventual generated model emits a
`static FIELDS` table beside a `Fields` impl, so the two can't drift apart.

## Tests

Behavior is specified end-to-end by the fixture files under `tests/cases/`.
One file is one case — a template, some data, and the result — grouped into a
directory per topic, so `ls` reads as the spec's table of contents:

```
tests/cases/groups/01-a-group-renders-when-its-placeholder-is-present.txt
tests/cases/groups/02-a-group-is-dropped-whole-taking-its-separator-with-it.txt
tests/cases/nullability/04-a-non-null-field-needs-no-fallback.txt
tests/cases/nullability/05-an-alternative-after-a-quoted-literal-is-unreachable.txt
…
```

```
--- template
{{opera.title}}[ ({{year}})]
--- data
opera.title = Salome
--- expect
Salome
```

Errors are captured the same way, diagnostic text and all:

```
--- template
{{composer.surname}}
--- error
error[template_unknown_field]: unknown field `composer.surname`
{{composer.surname}}
  ^^^^^^^^^^^^^^^^
help: Fields on `composer`: fullName, firstName, lastName, born, died.
```

Any text before the first `--- ` is a note, for what the file name could not
say on its own.

```bash
cargo test                       # run every case
UPDATE_EXPECT=1 cargo test       # rewrite expectations, then read the diff
```

A blessing run that changed anything fails on purpose, listing the files it
touched — a green test always means the expectations on disk are the ones
that ran. The format is documented at the top of `tests/e2e.rs`.

There are no unit tests. Everything the lexer and parser do is reachable
through `Template::parse` and `render`, so the fixtures are the only
specification — checked by mutating the lexer and confirming the suite
notices.

## The FindOpera model

`src/model/` supplies both halves of the seam for real recordings, and almost
all of it is generated from the GraphQL schema plus the query the CLI sends at
runtime:

```bash
cd codegen
npm install
npm run fetch-schema   # refresh the vendored schema
npm run generate       # rewrite src/model/generated.rs
```

Nullability is why this is generated rather than written. Every check above
depends on knowing which fields are always present, and deriving that from the
schema means it cannot be wrong — where asserting it by hand can be, in the
direction that turns a parse-time error back into a per-record failure.

FindOpera's schema declares nothing with `!`, so that knowledge comes from
`@semanticNonNull`, which marks a position as only ever null when the response
carries a matching error. It matches what the database holds: `opera.title`,
`composer.lastName` and `conductor.lastName` are annotated and measured 100%
populated, while `opera.englishTitle` is annotated on neither count and sits at
19.7%.

Some fields are built from others by a named rule rather than read off a
record. `composer.dates` and `conductor.dates` give `1685-1759`, or `b1947` for
someone still living; with no birth year they give nothing at all, since a lone
death year would need a spelling that means "died" and every candidate is
either awkward in a filename or reads as a negative number.

The output is checked in, so drift arrives as a reviewable diff. A field losing
its annotation upstream shows up as `FieldDoc::non_null` becoming
`FieldDoc::new` and `String` becoming `Option<String>` — which is exactly the
change that makes every bare `{{that.field}}` stop parsing. See
`codegen/README.md`.

```
schema/       schema.graphql, the query, and fields.mjs
src/template/ the language: lexer, parser, renderer, and the seam
src/model/    the recording model, mostly generated
src/api.rs    the GraphQL client
src/main.rs   the CLI
codegen/      derives the model from the schema and the query
```

## History

This was a CLI that fetched recordings from [FindOpera](https://findopera.com/)
and rendered them through these templates. It has been scoped back to the
templating core; the `full-linking-prototype` branch carries a larger earlier
version that also built trees of symlinks from marker files.
