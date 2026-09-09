# Several complaints each get their own line

A server may object to more than one thing at once, and each objection is
worth reading. They are laid out one to a line rather than run together.

## Answer Recordings

```json
{"errors":[{"message":"first"},{"message":"second"}]}
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
