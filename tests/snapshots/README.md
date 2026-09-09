# Snapshot cases

One case is two files:

- `some-case.md` — what to run. Written by hand, never generated.
- `some-case.expected.md` — what it produced. Generated, never edited.

A case names its sections, and the sections say what kind of case it is:

| Section | What it is |
|---|---|
| `## Library` | a `tree` block laid out as the source library |
| `## Destination` | files already there that this program did *not* build |
| `## Config` | the `findopera.toml`; the destination is filled in |
| `## Recordings` | serve the captured corpus, by whatever ids are asked for |
| `## Answer <Operation>` | a canned reply for one GraphQL operation |
| `## Run` | one `$ findopera …` line per command, run in order |

`## Destination` appears on both sides: as an input it is what was there
before, and in the generated file it is what is there after. It declares only
files nobody claims, which is the one starting state a run cannot produce —
for a destination this program built, run `organize --write` first, so the
tree and the record of it agree.

What comes back is `stdout`, `stderr`, `Destination`, `Requests` and `Exit`
— all of them, every time. The failures worth catching live *between* those:
a summary claiming a folder was built, above a destination that does not
contain it, is only visible if you can see both at once.

```bash
UPDATE_EXPECT=1 cargo test            # rewrite expectations, then read the diff
rm tests/snapshots/**/*.expected.md   # and rebuild them all from nothing
```

A blessing run that rewrote anything *fails*, so a green test always means
the expectations on disk are the ones that ran.
