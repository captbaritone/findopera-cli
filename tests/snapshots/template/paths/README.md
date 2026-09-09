# paths

Rendering is total: for any template `parse` accepted and any record, there is
a string. Judging that string as a relative path is the only thing left that
can fail per record, and this is where both halves of that live.

Some path problems are settled at parse time, because nothing about a record
can change them — a leading `/` written in the template, or a literal `..`
segment. Those carry `template_*` codes.

The rest need a value to provoke them and carry `path_*` codes: a group that
dropped and left a leading separator, or a value that sanitized away to
nothing. Note that separators written in the template are structural while
separators arriving inside a value are not, so an interpolated value is
confined to the single segment it was written into.
