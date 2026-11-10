A cached clean result of [`private-import-path`](../src/rules/imports/private_import_path.md) no longer stands once the
source module a test mirrors is deleted: a warm run reports the test's private import as a run without the cache does.
