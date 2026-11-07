`module-attribute-import` reports `module.Name` access again when another file reads an attribute of a module with the
same name through `__import__` or a plain `import`. Such a read used to suppress the report for every module of that
name.
