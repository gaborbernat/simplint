[`mutable-module-constant`](../src/rules/typing/mutable_module_constant.md) keeps a comment inside a `list[...]`
annotation when its fix makes it a `tuple[..., ...]`: it renames the base and adds the `, ...` instead of rewriting the
whole annotation.
