[import-cycle](../src/rules/imports/import_cycle.md) no longer treats a module-level flag as false when a block of the
module or a `global` declaration rebinds it, as `HAS_JSON = True` in a `try` after `HAS_JSON = False` does, so the
imports under the flag count in the cycle check.
