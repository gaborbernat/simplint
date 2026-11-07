`unused-export`, `unexported-public-name` and `unused-definition` no longer treat the hooks of the `[build-system]`
build backend, such as `build_wheel`, as unused or private. A build frontend calls those hooks by name, so they are
public names, as entry points are.
