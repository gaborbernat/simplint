`single-caller-helper` (SMP505) no longer suggests inlining a helper annotated `TypeGuard` or `TypeIs`, since the bare
expression would drop the narrowing its caller relies on.
