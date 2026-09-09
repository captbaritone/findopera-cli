# A destination with somebody else's files in it

<!-- From 03-a-destination-that-is-not-ours.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
```

## stderr

```
findopera: ./named already has things in it, and no record of this program having put them there.
    An empty folder, or one built by an earlier run, is what this can work with — otherwise there is no way to tell what would be safe to remove later.
```

## Destination

```tree
Their Folder/
Their Folder/notes.txt
someone-elses.flac
```

## Requests

```
Recordings
  {"ids":["75"]}
```

## Exit

```
2
```
