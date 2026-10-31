`simplint check` now behaves like `ruff check`: it prints reports in any of Ruff's output formats (`full` by default,
`concise`, `grouped`, `json`, `json-lines`, `junit`, `github`, `gitlab`, `pylint`, `rdjson`, `azure` and `sarif`), with
Ruff's summary lines and exit codes, and takes Ruff's flags: `--fix-only`, `--show-fixes`, `--statistics`,
`--exit-zero`, `--exit-non-zero-on-fix`, `--ignore-noqa`, `--output-file`, `--extend-select`, `--stdin-filename` or `-`,
and the `--quiet`, `--silent` and `--verbose` log levels. A file that does not parse or cannot be read is reported as an
`invalid-syntax` or `io-error` diagnostic, and the rest of the run goes on.
