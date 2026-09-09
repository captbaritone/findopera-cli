# A symlink follows the folder, not the files in it

<!-- From 03-a-symlink-follows-the-folder.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write
+ ./library/BillyBudd  Billy Budd
$ write ./library/BillyBudd/track02.flac 'a track added afterwards'
$ show './named/Billy Budd/track02.flac'
a track added afterwards
$ rm ./library/BillyBudd/track02.flac
$ show './named/Billy Budd/track02.flac'
<cannot read ./named/Billy Budd/track02.flac: entity not found>
```

## stderr

```
$ findopera organize ./library --write
findopera: building a link to each folder in ./named
findopera: 1 built, 0 already there, 0 left alone
```

## Destination

```tree
Billy Budd -> …/BillyBudd
```

## Requests

```
Recordings
  {"ids":["75"]}
```

## Exit

```
0
```
