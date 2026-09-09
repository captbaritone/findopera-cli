# The server refuses the lookup

A GraphQL server reports its complaint in the body of a perfectly successful
response, so this is what a client that only looked at the status would call
a success. It is reported as a refusal, with the server's own words, and
nothing is planned from it.

The answer here is written out rather than taken from the captured
recordings, which is what a case does when the interesting part is a reply
nothing real would give it.

## Answer Recordings

```json
{"errors":[{"message":"This version of findopera-cli is no longer supported. Please upgrade.","extensions":{"code":"CLIENT_TOO_OLD"}}]}
```

## Library

```tree
DonGiovanni/findopera-332.txt
```

## Config

```toml
template = "{{opera.title}}"
```

## Run

```console
$ findopera organize ./library --tabs
```
