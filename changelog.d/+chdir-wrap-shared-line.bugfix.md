[up-chdir-restore](../src/rules/logic/up_chdir_restore.md) reports without a fix when the assignment shares its line
with an earlier statement. The fix used to leave that statement on the `with` line, which does not parse.
