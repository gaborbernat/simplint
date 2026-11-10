[`dependency-import-guard`](../src/rules/imports/dependency_import_guard.md) no longer reports guards around
platform-only and optional standard library modules, such as `_overlapped` or `mmap`, or around CPython's `_test*`
extensions. Its unsafe fix no longer adds a suppression to a function-local import that the function uses.
