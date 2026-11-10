`--add-ignore` no longer adds `numeric-ignore` to a comment whose numeric codes it renames, which left a selector that
[`unused-ignore`](../src/rules/comments/unused_ignore.md) (SMP009) then reported.
