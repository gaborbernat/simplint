Without a `pyproject.toml`, a module under a `test` or `tests` package now counts as a test module wherever it sits in
that package, so helpers and data files there get the test-only rules, such as `test-module-docstring` (SMP902), and
their uses count as test uses for rules such as `test-only-code` (SMP408). The settings root is the directory the run
starts from, as Ruff roots a project without configuration at the working directory, instead of each file's directory.
