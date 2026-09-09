# What each way of linking actually does

A hard link, a clone and a copy are all just files in a listing, and their
inode numbers are not what anybody chose between them for. These cases ask
the question that tells them apart instead: write through the one in the
destination, and read the one in the library.

There is no case for a clone. It behaves like a copy in every way a person
can observe — which is exactly the point of it — so what separates them
lives in `tests/apply.rs`, where an inode is the only thing left to look at.
