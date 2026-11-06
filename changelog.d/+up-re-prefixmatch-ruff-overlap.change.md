`up-re-prefixmatch` (SMP522) no longer reports a `re.match` call with a plain-text pattern whose result is only tested
for truth when the project's Ruff configuration enables preview mode and selects
[RUF055](https://docs.astral.sh/ruff/rules/unnecessary-regular-expression/), which rewrites it to `str.startswith`.
