Docstring fixes (SMP901, SMP902) no longer rewrite a file with a file-level `ruff: noqa`, `flake8: noqa` or
`ruff: file-ignore`. Removing or reflowing the docstring could leave the codes of that directive unused, which Ruff then
reports as RUF100.
