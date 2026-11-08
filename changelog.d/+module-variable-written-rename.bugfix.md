`module-variable-case` and `module-variable-prefix` keep their report but offer no rename for a module variable that
another module assigns as an attribute, as `module.NAME = value` or `setattr(module, "NAME", value)` does, since the
rename would leave that write pointing at the old name.
