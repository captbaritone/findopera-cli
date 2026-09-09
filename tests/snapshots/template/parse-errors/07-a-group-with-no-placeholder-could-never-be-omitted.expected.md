# A group with no placeholder could never be omitted

<!-- From 07-a-group-with-no-placeholder-could-never-be-omitted.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: this group contains no placeholder of its own, so it would always render
  {{opera.title}}[ - ]
                 ^^^^^
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
