`dependency-import-guard` (SMP303) counts a submodule of a standard library package, such as `asyncio.taskgroups`, as
shipped only from the version that adds it, using a table generated from CPython's `Lib/` trees, so the guard around it
is reported on the targets that have it. Its unsafe fix removed an import left unused by the unwrap, such as
`import string.templatelib` checked only for existence, and keeps it now with a `noqa` suppression. The guards around
`_zstd`, `compression.zstd` and `_curses` stay unreported.
