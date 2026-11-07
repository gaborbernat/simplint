`string-concatenation` (SMP503) no longer reports a `+` chain with an instance of a class imported into the module, such
as `Markup("<b>") + "<i>"` with `markupsafe.Markup`, since `+` runs that class's own `__add__`. A class of the project
is recognized whatever its name, and an import from outside the project counts as a class when its name is CapWords.
