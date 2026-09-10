# A template left unquoted

<!-- From 14-a-template-left-unquoted.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: ./findopera.toml is not valid:
    TOML parse error at line 1, column 13
      |
    1 | template = {{opera.title}}
      |             ^
    missing key for inline table element, expected key

    hint: a template has to sit in a `'''` block, on its own lines:
              template = '''
              {{composer.lastName}}/{{opera.title}}
              '''
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
