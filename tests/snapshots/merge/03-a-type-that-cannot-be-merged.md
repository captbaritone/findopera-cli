# A type that cannot be merged is refused without asking

The server would answer a `mergeUpc` with a GraphQL error about a field that
does not exist, which says nothing about the type in hand. So it is settled
here, and the answer is the list of types that can be merged.

Nothing is scripted below, and the requests come back empty — which is the
other half of the claim: no round trip was spent finding out.

## Run

```console
$ findopera merge upc 1 --into 2 -m 'a reason' --yes
```
