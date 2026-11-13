[`mutable-module-constant`](../src/rules/typing/mutable_module_constant.md) leaves `pytestmark` and the other names
pytest reads by name alone: its fix made `pytestmark` a tuple, which pytest rejects as a single mark.
