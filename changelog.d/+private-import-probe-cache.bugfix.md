A test's private import is checked again when the source file it mirrors is deleted from a directory outside Ruff's
`src` roots, so a stale clean result no longer hides
[`private-import-path`](../src/rules/imports/private_import_path.md) (SMP310).
