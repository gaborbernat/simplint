[`unexported-public-name`](../src/rules/api/unexported_public_name.md) leaves alone a module that looks its own names up
by a computed key, as `globals()["do_" + name]` does, since its fix renamed the functions that lookup reaches.
