`up-removed-ast-node` (SMP520) flags `ast.Num`, `ast.Str`, `ast.Bytes`, `ast.NameConstant` and `ast.Ellipsis` references
and `visit_` methods that Python 3.14 removed, with safe fixes for the common `isinstance` and constructor forms.
