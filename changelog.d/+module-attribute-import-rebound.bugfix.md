`module-attribute-import` (SMP304) no longer moves a name that a function rebinds through `global`, or that a file of
the run stores on the module, into an import. The import would keep the value the module had when it loaded, so the
rebinding would not reach the importer.
