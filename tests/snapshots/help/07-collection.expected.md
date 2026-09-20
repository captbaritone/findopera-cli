# `findopera collection --help`

<!-- From 07-collection.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
A list of recordings you keep on findopera.com.

  findopera collection list
  findopera collection show 1
  findopera collection create 'Main library'
  findopera collection add 264 1247 --to 3
  findopera collection remove 264 --from 3

Every one of these names a list by its id rather than by its name, because
nothing stops two lists sharing a name. `collection list` is where an id comes
from; it says how much each one holds, and `collection show` prints what that
is, so neither question means opening the website.

  findopera collection show 3
  findopera collection show 3 --offset 50

`show` pages, and `search` does not, which is the difference between the two
ways of reading a list. A search is narrowed until the answer fits; walking a
list you already chose has to be able to reach the end of it.

Ids can be read from standard input by passing `-` in their place, which is
what makes this worth scripting:

  findopera search recording tosca --tabs | cut -f1 | findopera collection add - --to 3

None of this is versioned. The catalog keeps every version of a record, and
that is what `undelete` puts back; a list is a row changed in place, so taking
a recording out, renaming a list, replacing what it holds and purging it are
all final in the same way. There is nothing to undo any of them with.

`set` and `purge` are the two that can lose a great deal in one go, so both
say what they are about to do and ask for --yes: `set` whenever it would leave
a list holding less than it found, `purge` always. Purging never touches the
recordings themselves — they were never owned by the list.

Adding and removing are both idempotent, by the server's own account: adding
what is already there is one save, and removing what was never there is not an
error. So a run that died halfway through can simply be run again.

`collection set` replaces a list's contents outright, in one request rather
than one per difference — which is what something that has walked a whole
library and knows the answer wants. It is also the only one here that
destroys: what it does not name stops being in the list. Emptying a list that
way needs --yes.

All of these need a token, because a list belongs to an account.

Usage: findopera collection <COMMAND>

Commands:
  list    The lists this account keeps, and how much each holds
  create  Start a list
  add     Put recordings in a list
  remove  Take recordings out of a list
  set     Make a list hold exactly these recordings, and nothing else
  rename  Give a list a different name
  purge   Delete a list for good. The recordings in it are untouched
  show    What a list holds
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')
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
