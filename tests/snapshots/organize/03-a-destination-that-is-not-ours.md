# A destination with somebody else's files in it

Nothing here says this program built these, so there would be no safe answer
later about what may be removed. An empty folder, or one whose record we
wrote, are the only two it will accept.

Note what the destination looks like afterwards: untouched, including the
file that is not ours. Refusing is only half the promise — the other half is
that refusing costs nothing.

## Library

```tree
BillyBudd/findopera-75.txt
```

## Destination

```tree
someone-elses.flac
Their Folder/notes.txt
```

## Config

```toml
template = "{{opera.title}}"
link = "copy"
```

## Run

```console
$ findopera organize ./library --write
```
