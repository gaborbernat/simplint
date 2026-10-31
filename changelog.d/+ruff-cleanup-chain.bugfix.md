Ruff's cleanup after a fix also takes in the rules its own fixes give new work, such as the import sort after an import
moves to the top, and a fix that would make `ruff format` split a line carrying a `noqa` or other pragma comment is
withdrawn, since the split moves the comment off the code it applies to.
