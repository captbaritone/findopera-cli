# A link will not displace what is already there

<!-- From 31-a-link-will-not-displace-a-folder.md. -->
<!-- Generated. Do not edit; UPDATE_EXPECT=1 cargo test -->

## stdout

```
$ findopera organize ./library --write
+ ./library/BillyBudd  Billy Budd
$ rm './named/Billy Budd'
$ write './named/Billy Budd/something-elses.flac' 'a real folder, not a link'
$ findopera organize ./library --write
! ./library/BillyBudd  Billy Budd
$ show './named/Billy Budd/something-elses.flac'
a real folder, not a link
```

## stderr

```
$ findopera organize ./library --write
findopera: building a link to each folder in ./named
findopera: 1 built, 0 already there, 0 left alone
$ findopera organize ./library --write
findopera: building a link to each folder in ./named
findopera: ./named/Billy Budd
    a folder is already there
findopera: 0 built, 0 already there, 1 left alone
```

## Destination

```tree
Billy Budd/
Billy Budd/something-elses.flac
```

## Requests

```
Recordings
  {"ids":["75"]}
Recordings
  {"ids":["75"]}
```

## Exit

```
0
1
```
