# A list template that does not parse

<!-- From 04-a-list-template-that-does-not-parse.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: ./findopera.toml is not valid:
    the list template is not valid: unknown field `singers.surnames`
      {{opera.title}} — {{singers.surnames}}
                          ^^^^^^^^^^^^^^^^
      help: Fields on `singers`: lastNames.
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
