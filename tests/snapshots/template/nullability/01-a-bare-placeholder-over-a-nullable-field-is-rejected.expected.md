# A bare placeholder over a nullable field is rejected

<!-- From 01-a-bare-placeholder-over-a-nullable-field-is-rejected.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: {{year}} may be absent, and is not inside a group
  {{composer.lastName}}/{{year}}
                        ^^^^^^^^
  help: Add a fallback like {{…|"Unknown"}}, or wrap it in a group so it can be dropped: [{{year}}]
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
