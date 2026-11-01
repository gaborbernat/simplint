`missing-from-all` (SMP113) no longer asks to list in `__all__` a name the module imports and uses itself, such as `sys`
or `Path`, when another module reaches it through this one. An import the module never uses still counts as a re-export.
