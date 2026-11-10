A `.ruff_cache` directory, or the `simplint` directory in it, that a link in the repository points outside the project
no longer receives simplint's cache, its `CACHEDIR.TAG` or its `.gitignore`; the run warns and does not cache.
