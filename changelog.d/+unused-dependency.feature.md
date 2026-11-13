[`unused-dependency`](../src/rules/imports/unused_dependency.md) reports a requirement in `[project] dependencies` that
no file of the run outside the tests imports. Every user installs it for nothing, and one the tests alone import belongs
in a dependency group. The reports sit at the start of the `__init__.py` of the project's first top-level package.
