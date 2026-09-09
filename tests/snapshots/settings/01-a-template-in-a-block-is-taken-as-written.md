# A template in a block is taken as written

The apostrophe and the escaped bracket both survive, which is the whole reason
for the ''' form: what is in the file is what you would have typed.

## Toml

```toml
template = '''
{{opera.englishTitle|"L'inconnu"}} \[{{id}}\]
'''
```
