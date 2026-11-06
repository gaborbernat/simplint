`blanket-ignore` (SMP004) leaves the bare `noqa`, `ruff: noqa` and `flake8: noqa` comments to Ruff's PGH004, and bare
`type: ignore` and `pyright: ignore` to PGH003, when the project's Ruff configuration selects those rules. The `ty`,
`pyrefly`, `simplint` and `ruff: ignore` forms are still reported here.
