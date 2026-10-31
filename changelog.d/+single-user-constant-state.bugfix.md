`single-user-constant` no longer reports module values a function mutates, such as a set of warnings already shown or a
lock, and offers no fix for a constant with a trailing comment or a value that reads a name the module later deletes.
Its fix keeps the annotation of a new local, and inlines into an f-string only a plain name or number.
