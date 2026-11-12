[`private-import-path`](../src/rules/imports/private_import_path.md) again reports a private name imported from a
private module, such as `from ._api import _helper`. A module imports only public names, so the fix is to make the name
public in its private module. Importing a public name from a direct private child stays allowed.
