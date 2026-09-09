# A template left unquoted

TOML reads `{` as opening an inline table, so its own message never mentions
quoting. This is the one place its wording does not land, so a hint is added.

## Toml

```toml
template = {{opera.title}}
```
