`constant-assert` (SMP205) reads truthiness the way Ruff does, so it also reports assertions on non-empty list, set and
dict literals, f-strings and lambdas, such as `assert [value]`. A tuple holding only starred elements, which may be
empty, is no longer reported. An assertion that calls something, such as `assert [compute()]`, gets no fix.
