[`module-variable-case`](../src/rules/naming/module_variable_case.md),
[`unexported-public-name`](../src/rules/api/unexported_public_name.md) and
[`module-variable-prefix`](../src/rules/api/module_variable_prefix.md) keep their reports without a rename fix in a
project built with maturin or setuptools-rust, whose native code imports Python names simplint cannot see, as
cryptography's Rust code imports `serialize_ssh_public_key`.
