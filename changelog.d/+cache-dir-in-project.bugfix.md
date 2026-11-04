A `cache-dir` in `pyproject.toml` that points outside the project is ignored, and the cache goes to Ruff's default
directory inside it. Stale cache removal now touches only directories of other simplint builds, so a `cache-dir` can no
longer delete directories outside the project. The command line and `RUFF_CACHE_DIR` still choose any directory.
