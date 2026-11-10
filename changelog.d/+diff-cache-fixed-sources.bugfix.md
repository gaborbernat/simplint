A `--diff` run no longer caches the reports of a file linted after another file's fix, so the next `check` no longer
loses a [`duplicate-code`](../src/rules/logic/duplicate_code.md) report.
