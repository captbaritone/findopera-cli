# A leading separator would make the result absolute

<!-- From 08-a-leading-separator-would-make-the-result-absolute.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: template renders an absolute path
  /{{composer.lastName}}
  ^
  help: Drop the leading `/`; a rendered result is always relative.
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
