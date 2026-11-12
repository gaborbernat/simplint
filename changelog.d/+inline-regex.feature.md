[`inline-regex`](../src/rules/logic/inline_regex.md) reports a regular expression compiled inside a function, such as
`re.fullmatch(r"...", text)`, and its unsafe fix moves the pattern to a module-level constant compiled once. A call that
goes through `re` looks its pattern up in a bounded cache on every call, and a program that uses more patterns than the
cache holds compiles some of them again each time, so the constant keeps those calls fast.
