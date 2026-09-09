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
| `## Run` | one `$ …` line per step, run in order |
| `## Requires` | what the platform must offer — `unix`, so far |

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

A step in `## Run` is either the program or something done to the disk:

| Step | What it does |
|---|---|
| `findopera …` | run the command |
| `write <path> <text>` | put a file there |
| `append <path> <text>` | add to one |
| `rm <path>` | take one away |
| `link <target> <path>` | leave a symlink, as an earlier run might have |
| `show <path>` | read one back, onto stdout |

Paths start `./library/` or `./named/`, and want quoting where they contain a
space.

Those exist because a hard link, a clone and a copy all look identical in a
listing, and their inode numbers are not what anybody cares about. What
differs is what happens next — writing through one changes the original, or
does not; a track added later shows up through a folder link, or does not. So
a case changes a file and reads another, and the difference is what it reads
back.

A clone has no case of its own for exactly this reason: it behaves like a
copy in every way a person can observe, which is the point of it. What
separates it lives in `tests/apply.rs`, where an inode is the only thing left
to look at.

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
