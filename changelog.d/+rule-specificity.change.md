When `select` and `ignore` both match a rule, the more specific entry wins, as in Ruff: `--select SMP001 --ignore SMP0`
keeps `SMP001`. Before, `ignore` always won.
