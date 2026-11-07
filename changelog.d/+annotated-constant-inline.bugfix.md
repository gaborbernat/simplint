`single-user-constant` keeps an annotated constant as an annotated local instead of inlining its literal, so a value
such as a dict under `Final[dict[str, list[str]]]` keeps the type it declares and still type checks.
