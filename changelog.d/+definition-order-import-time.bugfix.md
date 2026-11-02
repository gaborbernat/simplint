`definition-before-use` (SMP020) no longer moves a helper below a lambda that runs while the module loads, such as
`(lambda: helper())()` or a callback passed to a module-level call, or below a class body that calls one of its own
functions which reads the helper.
