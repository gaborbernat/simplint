`module-attribute-import` (SMP304) and `missing-final` (SMP601) count a `setattr` or `patch.object` in a loop over a
literal tuple or list of modules as patching the name on each module the loop names. Such a module keeps its attribute
on the module instead of being reported.
