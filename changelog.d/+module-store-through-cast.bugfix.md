[`single-user-constant`](../src/rules/layout/single_user_constant.md) and
[`module-variable-prefix`](../src/rules/api/module_variable_prefix.md) count `cast(T, module).name = value` as a write
to `module.name`, so they no longer move or rename a variable another module sets that way, as fastapi's tests set a
tutorial module's file path.
