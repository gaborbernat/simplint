`walrus-candidate` (SMP502) no longer offers a fix for an assignment whose `if` is the only statement of an outer `if`
with a comment in its header, as Ruff would not merge the two past that comment. `--fix` also applies the other
assignments of the file, which it had withdrawn along with that one.
