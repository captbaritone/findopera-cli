# A setting given the wrong kind of value

<!-- From 12-a-setting-given-the-wrong-kind-of-value.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## Result

```
findopera.toml is not valid:
TOML parse error at line 2, column 20
  |
2 | require-variants = "yes"
  |                    ^^^^^
invalid type: string "yes", expected a boolean
```
