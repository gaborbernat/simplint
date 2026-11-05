`module-attribute-import` (SMP304) keeps the `lazy` keyword when its fix rewrites a lazy import, so the module still
loads on first use of a name. Its fix is no longer offered when a parameter, local, comprehension target, match capture
or type parameter of the same name would take the bare name, which made `make[Item]` call a `TypeVar`.
