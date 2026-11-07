`module-variable-prefix`, `module-variable-case`, `unexported-public-name`, `single-user-constant` and
`module-attribute-import` keep their reports but offer no fix for a module that a write under a computed name may set,
such as `setattr(module, name, value)` or `mock.patch(f"pkg.module.{name}")`. Renaming, moving or inlining its variables
would break that write.
