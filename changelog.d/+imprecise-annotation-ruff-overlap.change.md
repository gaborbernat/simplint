`imprecise-annotation` (SMP602) leaves an annotation that is or starts with `Any`, as in `Any | None`, to Ruff's ANN401
only when the project's Ruff configuration selects it; without ANN401 it now reports those annotations too.
