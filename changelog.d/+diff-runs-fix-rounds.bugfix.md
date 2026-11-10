`--diff` runs the fix rounds that `--fix` runs, so its diff shows the changes a later round makes, such as a
[`dataclass-frozen`](../src/rules/classes/dataclass_frozen.md) fix that becomes safe once a rename leaves the class with
no other user. Applying the diff gives the files `--fix` writes.
