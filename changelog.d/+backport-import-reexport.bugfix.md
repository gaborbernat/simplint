[`up-backport-import`](../src/rules/imports/up_backport_import.md) no longer deletes an `exceptiongroup` import of a
name that another module of the run imports from the same module or `__all__` lists. It still reports the import.
