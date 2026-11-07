`imprecise-annotation` (SMP602) no longer reports an `Any` that [ANN401](https://docs.astral.sh/ruff/rules/any-type/)
reports when the project's Ruff configuration selects it, now also inside `Optional`, `Union`, `|` and `Annotated`.
While ANN401 runs, it reports the `Any` ANN401 skips, such as `self: Any`, `@override` signatures and, under
`allow-star-arg-any`, `*args: Any`.
