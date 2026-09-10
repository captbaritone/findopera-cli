# A list template left unclosed

The other way a template fails: not a field that is not there, but a group
that was opened and never shut. The answer points at where it was opened,
since that is the end you have to go back to.

## Library

```tree
Mozart/findopera-332.txt
```

## Toml

```toml
template = '''{{opera.title}}'''

[index]
file = "list.txt"
template = '''{{opera.title}}[ ({{year}})'''
```

## Run

```console
$ findopera organize ./library
```
