A fix that Ruff's cleanup undoes, such as `absolute-sibling-import` in a project that bans relative imports, no longer
counts as a fix: the file is left alone and `--diff` and `--exit-non-zero-on-fix` see no change.
