`import-cycle` (SMP308) reports a runtime import that closes a cycle of modules importing each other when they load. The
report names the cycle and sits on the import that closes it. Imports in a function, under `if TYPE_CHECKING:`, a
`__main__` guard, a false constant or a version check the project's target never passes, and `except ImportError:`
fallbacks stay out of it, as do test modules.
