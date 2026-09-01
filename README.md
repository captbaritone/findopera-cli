# findopera

Name a music library from FindOpera metadata, through a template.

## Getting started

```bash
findopera init ~/Music     # write a findopera.toml, every setting explained
findopera scan ~/Music     # see what each folder would be called
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

Nothing searches up the tree. `scan` reads the file beside what you scanned, or
the one you name with `--config` — so one library can carry several, one per
way of naming it, and which one ran is never a guess:

```bash
findopera scan ~/Music --config ~/Music/by-conductor.toml
findopera scan ~/Music -t '{{opera.title}}'      # or just override the template
```

## Scanning

`findopera scan` walks a directory for marker files and shows what each folder
would be called:

```bash
$ findopera scan '{{composer.lastName}}/{{opera.title}}[ ({{year}})]' ~/Music
./Box Sets/Donizetti box      Donizetti/L'elisir d'amore (1969)
./Box Sets/Donizetti box      Respighi/Maria Egiziaca (1980)
./Britten/Billy Budd (Decca)  Britten/Billy Budd (1967)
./Handel - Sosarme 2026       Handel/Sosarme, Re di Media (2026)
```

A marker is a `.txt` file whose **name** carries a `findopera-<id>` token,
saved into the recording's folder:

```
findopera-10655.txt
Sosarme, Re di Media-2026 [findopera-10655].txt    the site's suggested name
```

A bare `10655.txt` is deliberately not enough — a number and a `.txt` is what
a track listing, a year or a disc number looks like, and the token is the part
that says the number means a recording.

What a marker identifies is the directory holding it, and one directory can
hold several: a box set covering several operas is listed once per recording,
as above.

Nothing opens the files. Deciding by content would mean reading every `.txt`
in the library to learn that almost none are markers — on a 12,500-file tree
that is 246ms against 66ms, nearly all of it wasted, and far worse over a
network mount where opening a file costs so much more than listing one. It
also keeps the contents from being load bearing: a marker may be empty, so
`touch findopera-10655.txt` works.

Results go to stdout, one line per recording; everything else to stderr.
`--tabs` separates the columns with a tab instead of padding, for piping.

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
$ findopera scan '{{opera.title}} \[{{id}}\][ ({{variant}})]' ~/Music
./rips/flac  Don Giovanni [332] (flac)
./rips/mp3   Don Giovanni [332] (mp3)
./billy      Billy Budd [75]
```

Where no variant was given and two directories still want one name, `scan`
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
$ findopera fields    # the syntax, and every field with whether it is always there
```

A bad template never costs a network round trip — it is checked against the
schema first:

```bash
$ findopera scan '{{composer.lastName}}/{{year}}' ~/Music
findopera: {{year}} may be absent, and is not inside a group
  {{composer.lastName}}/{{year}}
                        ^^^^^^^^
  help: Add a fallback like {{…|"Unknown"}}, or wrap it in a group so it can be dropped: [{{year}}]
```

| Exit | |
|---|---|
| 0 | everything rendered |
| 1 | a recording is missing, its render is not a usable path, or two directories want the same name |
| 2 | the template or the arguments are wrong |
| 3 | the API was unreachable or errored |

## The template language

A small language for turning record metadata into path-safe names.

```rust
let tmpl = Template::parse("{{composer.lastName}}/{{opera.title}}[/{{year}}]", FIELDS)?;
let rendered = tmpl.render(&recording);      // "Handel/Sosarme, Re di Media/2026"
to_path(&rendered)?;                         // ["Handel", "Sosarme, Re di Media", "2026"]
```

`render` is total — no `Result`. For any template this crate parsed and any
record, there is a string. Whether that string is a usable relative path is a
separate question, and the only one still decided per record.

## The language

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
