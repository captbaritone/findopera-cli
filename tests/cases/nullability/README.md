# nullability

What `FieldDoc::non_null` buys. Knowing which fields are always present lets
`parse` decide, with no data in hand, whether a placeholder may be absent — turning a failure on whichever record first lacks a field into an
error on the template itself.

Three checks live here: a placeholder that may be absent with nowhere to be
dropped, an alternative that can never be reached, and a group that can
never be dropped.
