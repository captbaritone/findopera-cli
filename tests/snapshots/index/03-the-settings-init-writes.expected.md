# The settings `init` writes

<!-- From 03-the-settings-init-writes.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera init ./library
$ findopera organize ./library --tabs
./library/Mozart	Mozart/Don Giovanni/1959 Maazel [332]
```

## stderr

```
$ findopera init ./library
findopera: edit the template in there, then run `findopera organize`
$ findopera organize ./library --tabs
findopera: no destination set, so these are only the folder names. Add one to build it:
    destination = "/path/to/named"
```

## Destination

```tree
```

## Requests

```
Recordings
  {"ids":["332"]}
```

## Exit

```
0
0
```
