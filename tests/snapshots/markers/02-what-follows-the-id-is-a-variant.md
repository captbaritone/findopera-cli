# What follows the id is a variant

A word after the id says which rip this is, and `{{variant}}` picks it up.
The delimiters around the id are not part of it: a bracket, a dot or a dash
belongs to the name the file arrived with, not to the word you chose.

A bare id carries no variant at all, so the group holding it drops.

## Recording 332

```json
{"opera": {"title": "Don Giovanni"}}
```

## Library

```tree
space/findopera-332 flac.txt
dash/findopera-332-DSD.txt
bracketed/Don Giovanni [findopera-332] SACD.txt
parens/findopera-332 (LP).txt
bare/findopera-332.txt
```

## Run

```console
$ findopera organize ./library --tabs -t '{{opera.title}}[ ({{variant}})]'
```
