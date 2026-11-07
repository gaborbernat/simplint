`unexported-public-name` and `unused-export` no longer rename or drop a name that a script reads by its string key off
the globals `runpy.run_path` or `runpy.run_module` returns, such as `runpy.run_path(path)["main"]`, when the path or
module names a project module. A key that no static path names counts as a read of any module's name.
