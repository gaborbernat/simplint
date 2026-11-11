[`dependency-import-guard`](../src/rules/imports/dependency_import_guard.md) no longer reports guards around
`asyncio.unix_events`, `multiprocessing.popen_forkserver` or the C accelerators CPython imports under a fallback, such
as `_json`, which a platform or build may lack. Guards around other private modules, such as `_thread`, are still
reported.
