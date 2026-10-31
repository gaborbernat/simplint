Removing the docstring of a test module that is left with comments only no longer leaves blank lines at the end of the
file, which `ruff format` keeps and Ruff reports.
