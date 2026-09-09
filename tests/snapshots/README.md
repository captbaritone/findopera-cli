# Snapshot cases

One case is two files:

- `some-case.md` — what to run. Written by hand, never generated.
- `some-case.expected.md` — what it produced. Generated, never edited.

A case names its sections, and they say what it needs:

| Section | What it is |
|---|---|
| `## Library` | a `tree` block laid out as the source library |
| `## Destination` | files already there that this program did *not* build |
| `## Config` | settings, with the destination filled in for you |
| `## Toml` | settings written exactly as given, for a case about the file itself |
| `## Recordings` | serve the captured corpus, by whatever ids are asked for |
| `## Recording <id>` | one recording, said by how it differs from a real one |
| `## Answer <Operation>` | a canned reply for one GraphQL operation, written out |
| `## Run` | one `$ findopera …` line per command, run in order |

`## Destination` appears on both sides: as an input it is what was there
before, and in the generated file it is what is there after. It declares only
files nobody claims, which is the one starting state a run cannot produce —
for a destination this program built, run `organize --write` first, so the
tree and the record of it agree.

Three ways to answer findopera.com, in increasing order of how much a case
has to say. `## Recordings` names ids and the captured corpus supplies them.
`## Recording <id>` folds a few fields over a captured one, so a case about a
title says only the title — written out in full it would have to satisfy the
whole generated model, and a hand-written record that drifts from the schema
is a fixture describing a server that does not exist. `## Answer <Operation>`
replaces the reply entirely, for the cases whose subject is an answer nothing
real would give: a refusal, a rate limit, a record that is not there.

An `## Answer` beats the corpus. A case that writes one out means it.

Every case runs a command and is judged on what a person would see. A
behaviour that cannot change anything visible is not a case: it can neither
break anybody nor be noticed doing so — which is why the settings cases show
the folder a template renders, or the refusal a contradiction draws, rather
than the fields a file parsed into.

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
