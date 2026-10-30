# simplint

A companion linter for [Ruff](https://github.com/astral-sh/ruff) that enforces the rules of the `simp` house style Ruff
cannot express. It is built on Ruff's own parser crates, so it is as fast as Ruff, and it fixes violations where the fix
is mechanical.

```console
simplint check src tests
```
