# Working out what each folder should be called

What a template makes of a library, before anything is built. These run
`organize` with no destination, so the plan is all there is: the listing on
stdout, and anything wrong with it on stderr.

Three things they cover, in order:

- what a marker means — the directory that holds it, a box set listed once
  per recording, a group dropping around a value that is not there;
- what happens when two folders want the same name — a variant declared in
  the marker, a number taken from walk order when none is, and
  `--require-variants` turning that number into a refusal;
- what cannot be built at all — an id the database does not have, a name
  that is not a usable path, and a folder that would land inside another
  recording's folder.

The exit code is where the last of those shows: numbering is a warning at
`0` and a refusal at `1`, and nothing else about the run says which.
