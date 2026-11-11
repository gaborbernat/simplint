[`single-use-variable`](../src/rules/logic/single_use_variable.md) finds the `=` debug fields that an edit lies in with
a search over the fields of the file, and builds its groups of edits only for the fields that get one. A file of 20,000
inlinable pairs and 20,000 `f"{s=}"` fields takes 0.10 seconds under SMP504 instead of 1.7.
