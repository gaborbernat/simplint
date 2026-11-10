[`absolute-sibling-import`](../src/rules/imports/absolute_sibling_import.md) (SMP302) looks for a flake8-lazy hook only
up to the linted project when no repository holds it, so a `.pre-commit-config.yaml` or `prek.toml` in the home
directory no longer turns the rule off. A hook configuration or a `hatch.toml` that cannot be read or parsed counts as
absent, with a warning, instead of stopping the run.
