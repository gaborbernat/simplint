Files and the target Python version now come from Ruff's own configuration resolution: `--config`, `--isolated`,
`extend` chains, user-level Ruff configuration, Ruff's built-in excludes, `lint.exclude`, `respect-gitignore`,
`force-exclude` and `per-file-target-version` all apply, and `--target-version`, `--exclude`, `--extend-exclude`,
`--respect-gitignore` and `--force-exclude` override them. Without any version setting the target is Ruff's default,
Python 3.14, instead of 3.11. `[tool.simplint]` also takes `fix`, `fix-only`, `unsafe-fixes`, `show-fixes` and
`output-format`, and rejects unknown keys.
