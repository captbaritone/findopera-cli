# Data alongside an error is not enough

Partial data cannot be trusted. A `null` where the schema promises a value
is explained by exactly one of the errors that came with it, so a response
carrying both is a refusal rather than a half-answer to work from.

## Answer Recordings

```json
{"data":{"getRecordingByIds":[null]},"errors":[{"message":"partial"}]}
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
