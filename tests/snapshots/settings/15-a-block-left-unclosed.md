# A block left unclosed

The column is past the end of the last line because the file ends there,
with the block still open. It ends with a newline, as a file written by a
person does — which is what moves the caret one further along than a input
with no final newline would.

## Toml

```toml
template = '''
{{opera.title}}
```

## Library

```tree
DonGiovanni/findopera-332.txt
```

## Run

```console
$ findopera organize ./library
```
