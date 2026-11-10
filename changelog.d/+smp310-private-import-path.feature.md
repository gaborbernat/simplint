[`private-import-path`](https://simplint.readthedocs.io/en/latest/_generated/rules/SMP310.html) reports private
module-object imports and private imports outside a direct relative path, including nested names such as `._x._y`. Tests
may import names from a private module only when their package path and `test_` stem match the source module.
