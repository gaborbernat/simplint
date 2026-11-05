Repeated runs without `--fix` reuse the cached reports of files that did not change, rather than only skipping files
that had none, and the project index resolves each imported module once per directory.
