# A group whose only placeholder sits in a nested group

<!-- From 08-a-group-whose-only-placeholder-sits-in-a-nested-group.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: this group contains no placeholder of its own, so it would always render
  [ - [{{year}}]]
  ^^^^^^^^^^^^^^^
  help: A group `[…]` is dropped when a placeholder inside it turns out to be absent. For a literal bracket, write `\[` and `\]`.
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
