[`private-import-path`](../src/rules/imports/private_import_path.md) reports a public name imported under a private
alias, such as `from ._api import canonical as _canonical`, and its unsafe fix imports the name as itself.
