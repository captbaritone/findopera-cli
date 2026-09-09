# An error with no message still says something

The message is optional in the protocol and mandatory in practice: an empty
complaint would be reported as an empty line, which reads as nothing having
gone wrong. So the code is shown, and the absence is stated.

## Answer Recordings

```json
{"errors":[{"extensions":{"code":"WEIRD"}}]}
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
