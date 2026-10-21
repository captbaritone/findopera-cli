# `findopera policy --help`

<!-- From 09-policy.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
Print the editorial policy the catalog is edited under, fetched from
findopera.com. Read it before a first edit.

  findopera policy             the whole of it
  findopera policy R12         one rule
  findopera policy P1          one procedure
  findopera policy identity    every rule in one stage

Usage: findopera policy [OPTIONS] [RULE]

Arguments:
  [RULE]
          A rule (R12, or 12), a procedure (P1) or a stage (identity). Omit for the whole policy

Options:
      --token <TOKEN>
          Token to identify as. Overrides the environment and the stored one

      --endpoint <URL>
          GraphQL endpoint. The policy is fetched from the same server
          
          [default: https://findopera.com/api/graphql]

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
