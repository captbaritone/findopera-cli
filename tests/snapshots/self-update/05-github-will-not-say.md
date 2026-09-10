# GitHub will not say

Anonymous callers get a limited number of these an hour from one address, and
meet this often enough that `403` on its own would send somebody looking for
a permission they do not need. So the limit is named, and the exit code says
the run did not get its answer.

## Answer 403

```json
{"message":"API rate limit exceeded"}
```

## Run

```console
$ findopera self update
```
