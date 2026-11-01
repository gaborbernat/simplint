Imports resolve with `ruff_graph`'s module database and Ruff's import helpers, so a file's package follows Ruff's
package detection, including `namespace-packages`, and a relative import reaching above the top-level package resolves
to nothing, as in Python.
