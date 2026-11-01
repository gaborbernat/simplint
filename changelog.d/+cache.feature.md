Like Ruff, `simplint check` caches which files had nothing to report, and with them the import facts of each file, so
later runs skip unchanged files. A cached result is reused only while the file, its settings and the names other files
use from it are unchanged. The cache lives in Ruff's cache directory, `.ruff_cache` unless Ruff's `cache-dir`,
`--cache-dir` or `RUFF_CACHE_DIR` names another, under `simplint/<version>`, next to Ruff's own files, which it leaves
alone. It is a compact binary file per package, written only when it changed, and keeps files other runs checked for 30
days, as Ruff's does. `--no-cache` or `SIMPLINT_NO_CACHE` turns it off, and `simplint clean` removes simplint's part of
each `.ruff_cache`.
