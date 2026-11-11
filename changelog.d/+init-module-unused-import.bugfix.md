[`init-module-content`](../src/rules/api/init_module_content.md) (SMP407) no longer reports an import that nothing reads
in an `__init__.py`, which Ruff's F401 reports, and it keeps the statement when another name of it is not one F401
reports.
