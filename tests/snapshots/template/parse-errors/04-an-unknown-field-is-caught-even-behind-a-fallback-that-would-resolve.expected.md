# An unknown field is caught even behind a fallback that would resolve

<!-- From 04-an-unknown-field-is-caught-even-behind-a-fallback-that-would-resolve.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: unknown field `opera.subtitle`
  {{opera.subtitle|"Untitled"}}
    ^^^^^^^^^^^^^^
  help: Fields on `opera`: title, englishTitle, librettist, url, language.name, language.abbreviation, language.
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
