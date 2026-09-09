# An alternative after a non null field is unreachable

<!-- From 07-an-alternative-after-a-non-null-field-is-unreachable.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: this alternative is unreachable — `opera.title` is never absent
  {{opera.englishTitle|opera.title|"Untitled"}}
                                   ^^^^^^^^^^
  help: Remove it, or put it before the alternative that is always there.
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
