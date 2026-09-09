# A chain of only nullable fields is still rejected

<!-- From 02-a-chain-of-only-nullable-fields-is-still-rejected.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: {{opera.englishTitle|opera.librettist}} may be absent, and is not inside a group
  {{opera.englishTitle|opera.librettist}}
  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  help: Add a fallback like {{…|"Unknown"}}, or wrap it in a group so it can be dropped: [{{opera.englishTitle|opera.librettist}}]
  see `findopera template` for every field and the syntax
```

## Destination

```tree
```

## Requests

```
```

## Exit

```
2
```
