`--config`, `--isolated` and the new `--color` flag go before or after the subcommand, as in Ruff. `--config` also takes
`KEY = VALUE` overrides: a `[tool.simplint]` key, such as `--config "select = ['SMP1']"`, overrides simplint's setting,
and any other key Ruff's, such as `--config "target-version = 'py312'"`. `FORCE_COLOR` turns colors on as in Ruff.
