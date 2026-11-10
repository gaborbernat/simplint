`--fix` keeps the [import-cycle](../src/rules/imports/import_cycle.md) and
[duplicate-code](../src/rules/logic/duplicate_code.md) reports of a file it rewrote. Before, the rewritten file lost
them, and the run could exit 0 with the cycle still there.
