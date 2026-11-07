`unused-export` and `unexported-public-name` no longer remove or rename a name a module reads off a module loaded from a
file path with `importlib.util.spec_from_file_location`, when the path names a project module. Names read off a module
loaded by `importlib.import_module` and similar calls now count as imports, so `missing-from-all` and `unused-export`
see them too.
