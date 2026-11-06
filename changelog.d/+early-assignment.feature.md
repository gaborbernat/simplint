`early-assignment` (SMP526) reports a local assigned a constant, such as a number, string or a list of them, above the
statements that do not use it, when its first use comes further down the same block. Moving the assignment down to that
use keeps the value beside the code that reads it. It stays put in a function that reads names by their text, such as
with `exec`, `eval`, `locals` or `six.exec_`, and when a string between the assignment and the use spells the name.
