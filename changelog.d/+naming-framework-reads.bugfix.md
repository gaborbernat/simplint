`module-variable-case` (SMP102) and `unexported-public-name` (SMP103) keep the names PLY, PEP 249 and the `typer`
command read by name, and names read off modules loaded by a computed name, such as a test fixture's
`importlib.import_module`. Relative imports in namespace packages now count as imports, and `missing-from-all` (SMP113)
no longer lists a name that would shadow the module itself in its package's `import *`.
