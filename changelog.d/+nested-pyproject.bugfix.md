A nested `pyproject.toml` without a `[tool.simplint]` table no longer hides the settings of a parent directory, matching
how Ruff skips one without `[tool.ruff]`.
