A workspace member is now judged as a library or an application by its own `pyproject.toml` when that has a `[project]`
or `[build-system]` table, so a workspace root that only configures simplint no longer makes its library members
applications. The root's `[tool.simplint]` still sets the rules.
