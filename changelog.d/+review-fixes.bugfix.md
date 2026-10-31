The cache now accounts for `skip-magic-trailing-comma`; a relative `--config` path anchors per-file ignores at its
directory; a per-file-ignore pattern such as `./tests/*.py` matches as in Ruff; an empty rule selector is an error; a
file simplint cannot read is reported as `io-error` only while Ruff's `E902` is on, and otherwise logged as `ruff check`
does; and the "No Python files found" warning shows once.
