`blanket-ignore` (SMP004) leaves the `ruff: ignore` and `ruff: file-ignore` comments without codes to Ruff's
[RUF103](https://docs.astral.sh/ruff/rules/invalid-suppression-comment/), which reports them, when the project's Ruff
configuration selects it. The other forms are still reported here.
