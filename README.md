# findopera

A small template language for turning record metadata into path-safe names.

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

A `[…]` group is dropped whole when a placeholder inside it resolves to
nothing, which is how a separator vanishes along with the value it was
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
group is only accepted when some alternative always resolves, so there is no
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
tests/cases/groups/01-a-group-renders-when-its-placeholder-resolves.txt
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

## History

This was a CLI that fetched recordings from [FindOpera](https://findopera.com/)
and rendered them through these templates. It has been scoped back to the
templating core; the `full-linking-prototype` branch carries a larger earlier
version that also built trees of symlinks from marker files.
