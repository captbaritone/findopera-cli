# A group whose placeholders all resolve can never be dropped

<!-- From 06-a-group-whose-placeholders-all-resolve-can-never-be-dropped.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: every placeholder in this group is always present, so it would always render
  {{opera.title}}[ ({{year|"n.d."}})]
                 ^^^^^^^^^^^^^^^^^^^^
  help: A group `[…]` exists to be dropped. Drop the brackets, or use a field that can be absent.
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
