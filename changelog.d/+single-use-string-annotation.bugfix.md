`single-use-variable` (SMP504) counts a name that a string annotation or `ForwardRef` reads, so it no longer inlines a
`TypeVar` read only in `"list[T]"`, which left the annotation naming an undefined name.
