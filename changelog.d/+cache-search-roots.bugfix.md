Cached import facts are collected again when a file's module path changes, as when Ruff's `src` setting moves the search
roots, instead of resolving relative imports against the old path until the file changes.
