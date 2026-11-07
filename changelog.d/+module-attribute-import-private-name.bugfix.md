`module-attribute-import` (SMP304) no longer reports a read of a private name, such as `main._compute_max_logs`, when
Ruff's `import-private-name` (PLC2701) would flag the import it suggests, so the two rules no longer need a suppression
to agree.
