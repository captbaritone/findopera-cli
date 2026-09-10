# `findopera self update --help`

<!-- From 03-self-update.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
Ask GitHub whether a newer findopera has been published, and say how to get
it if so.

  findopera self update

It does not replace this binary. Whatever installed it — an installer, a
package manager, or you with `scp` — knows where the binary lives and what
belongs beside it, and is what should replace it. On the machines this is
usually run on, the binary may not even be writable by whoever runs it.

So this reports, and names the command to run. Which command that is depends
on how this copy looks to have been installed, which is guessed from where it
sits and said as a guess.

The check is one anonymous request to GitHub's releases API, which allows a
limited number of those an hour from one address.

Usage: findopera self update [OPTIONS]

Options:
      --json
          Print the result as JSON

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
