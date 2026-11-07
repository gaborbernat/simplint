`unused-export` no longer drops a name that a test patches through its dotted path, `mock.patch("pkg.mod.NAME")`, or
through the module object, `monkeypatch.setattr(module, "NAME", value)`, and `single-user-constant` no longer inlines a
constant that a string in the run spells, which would replace the value the test patches.
