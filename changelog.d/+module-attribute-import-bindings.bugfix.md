[module-attribute-import](../src/rules/imports/module_attribute_import.md) offers no fix that removes a module name
another module imports or reads. It also offers no fix for a Python builtin's name, such as `set`, since the import
would shadow the builtin.
