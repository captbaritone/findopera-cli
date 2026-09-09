# Building the tree, and taking it down again

What `--write` does: where it refuses to build, what it leaves alone, and
what it removes when a name stops being wanted.

The refusals come first, because they run before anything is touched — a
destination that overlaps the library, in either direction. Then the ordinary
behaviour: a dry run that does neither of the things it describes, a second
run that finds its own work and changes nothing.

The rest is the safety argument. Only what was written down is ever removed,
so a folder somebody put there by hand survives a run that orphans
everything; an entry that no longer matches its record is reported and left;
and a link left by an earlier run cannot become a way to write into the
library.
