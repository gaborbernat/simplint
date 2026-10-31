`module-variable-case` (SMP102) looks for the name in the text strings hold rather than in their source, so whether the
rename is offered no longer depends on escapes that `ruff format` rewrites, such as `\N{latin small letter x}`.
