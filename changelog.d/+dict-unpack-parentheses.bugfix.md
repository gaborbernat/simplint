[`up-comprehension-unpacking`](../src/rules/logic/up_comprehension_unpacking.md) removes the parentheses around a dict's
key and value too, so `{(key): (value) for entry in pairs for key, value in entry.items()}` becomes
`{**entry for entry in pairs}` instead of an invalid expression that reverted the whole file.
