`imprecise-annotation` (SMP602) no longer reports `object` as the return type of a `Callable`, such as
`Callable[..., object]`, which says that the result is ignored. `object` in its parameters is still reported.
