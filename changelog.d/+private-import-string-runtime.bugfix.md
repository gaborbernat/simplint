[`private-import-path`](../src/rules/imports/private_import_path.md) no longer reports a private import that the code
uses at runtime when a string type expression, such as `cast("_SubParsersAction", value)`, also names it: Ruff's
import-private-name (PLC2701) reports that import, so the two no longer both warn.
