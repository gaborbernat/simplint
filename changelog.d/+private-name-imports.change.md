[`private-import-path`](../src/rules/imports/private_import_path.md) reports a private name imported from another
module, such as `from ._api import _helper` or `from .public import _helper`. A module imports only public names, so the
fix is to make the name public in its module. Importing a public name from a direct private child stays allowed.
