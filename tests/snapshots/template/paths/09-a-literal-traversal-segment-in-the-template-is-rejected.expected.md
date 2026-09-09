# A literal traversal segment in the template is rejected

<!-- From 09-a-literal-traversal-segment-in-the-template-is-rejected.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: `..` is not a usable path segment
  {{composer.lastName}}/../{{opera.title}}
                       ^^^^
  help: A rendered result may not name a parent or the current directory. Remove the segment.
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
