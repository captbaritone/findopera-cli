# A misspelled setting is named with the real ones

<!-- From 10-a-misspelled-setting-is-named-with-the-real-ones.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## Result

```
findopera.toml is not valid:
TOML parse error at line 2, column 1
  |
2 | requre-variants = true
  | ^^^^^^^^^^^^^^^
unknown field `requre-variants`, expected one of `template`, `destination`, `link`, `require-variants`, `follow-links`, `ignore`
```
