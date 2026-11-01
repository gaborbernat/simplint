`walrus-candidate` (SMP100) and `missing-final` (SMP101) no longer run in a module with a version gate, a top-level `if`
on `sys.version_info` that raises or exits, since such a module runs on interpreters that lack `:=` or `typing.Final`.
