`__import__("pkg.mod")` returns the package `pkg`, so the names read off its result are read from `pkg` and not from
`pkg.mod`, unless its `fromlist` is a non-empty list or tuple, or an expression that may hold names. A submodule read
off its result, as in `__import__("pkg.mod").mod.name`, is read from `pkg.mod`.
