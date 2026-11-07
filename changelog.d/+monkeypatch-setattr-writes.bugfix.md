`missing-final` (SMP601) no longer reports a module variable that a `setattr` call sets on its module, such as
`scoped.setattr(paths, "OVERRIDE_PIPX_HOME", None)` in a test or `monkeypatch.setattr("pkg.paths.NAME", None)`, whatever
the object is named. A module that a `setattr` call may set under a computed name, such as
`setattr(paths, name, value)`, reports none of its variables, since any of them may be reassigned.
