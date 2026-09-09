# A template in a block is taken as written

The apostrophe and the escaped bracket both survive, which is the whole
reason for the ''' form: what is in the file is what you would have typed.
A '…' would end at the apostrophe, and a "…" would refuse \[ as an escape it
does not know.

The apostrophe you can see. The recording has no English title, so the quoted
fallback renders, and `L'inconnu` reaches the folder name intact.

The brackets you cannot, and it is worth saying why they are proved anyway.
Had the backslashes been eaten, the template would read `[{{id}}]` — a group
around a field that is always present, which the language refuses as a group
that could never be dropped. So this case rendering at all is the evidence:
`[332]` on its own would look the same either way. That refusal is pinned in
`tests/cases/nullability/`.

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
