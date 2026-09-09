# An empty list of errors is not a refusal

The specification says the field is absent unless there is something in it,
but a server sending `"errors": []` has not objected to anything. Reading
that as a refusal would fail a run over punctuation.

The recording is missing here for the ordinary reason — the answer says
`null` — and that is what gets reported, rather than anything about errors.

## Answer Recordings

```json
{"errors":[],"data":{"getRecordingByIds":[null]}}
```

## Library

```tree
BillyBudd/findopera-75.txt
```

## Config

```toml
template = "{{opera.title}}"
```

## Run

```console
$ findopera organize ./library
```
