# `findopera describe --help`

<!-- From 05-describe.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
With no argument, list every type these commands work on.

With one, say what that type holds and what it takes to make one:

  findopera describe singer
  findopera describe singer --json    a JSON Schema for the create input

The JSON form is a schema in the ordinary sense — draft 2020-12 — so anything
that already validates JSON can check an input before sending it.

Usage: findopera describe [OPTIONS] [TYPE]

Arguments:
  [TYPE]
          The type. Omit to list them all

Options:
      --json
          Print a JSON Schema for the create input

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
