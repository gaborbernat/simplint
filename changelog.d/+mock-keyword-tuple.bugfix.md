`mock-attribute-assignment` wraps a bare tuple, `:=` or `yield` value in parentheses when it moves it into the mock
constructor, so `return_value = "a", {}` no longer turns into two arguments.
