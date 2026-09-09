# A template in a block is taken as written

The apostrophe and the escaped bracket both survive, which is the whole
reason for the `'''` form: what is in the file is what you would have typed.

Which is only worth knowing if it reaches the folder name, so that is what
this shows. The recording has no English title, so the quoted fallback is
what renders — apostrophe and all — inside brackets that were escaped to
stay literal.

## Toml

```toml
template = '''
{{opera.englishTitle|"L'inconnu"}} \[{{id}}\]
'''
```

## Library

```tree
DonGiovanni/findopera-332.txt
```

## Run

```console
$ findopera organize ./library --tabs
```
