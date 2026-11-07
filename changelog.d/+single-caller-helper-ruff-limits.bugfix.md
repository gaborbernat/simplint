`single-caller-helper` (SMP505) no longer asks to inline a helper when the merged caller could cross the limit of Ruff's
`C901`, `PLR0912`, `PLR0915` or `PLR0911` while Ruff runs that rule; the inlined counts are estimated conservatively.
