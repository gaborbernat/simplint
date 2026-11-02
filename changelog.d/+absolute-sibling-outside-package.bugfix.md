`absolute-sibling-import` (SMP110) no longer reports imports in a module that compares `__name__` with a string, such as
a script with a `__main__` guard or an execnet `__channelexec__` module, nor imports inside a function passed to
`remote_exec` or `inspect.getsource`. That code runs outside its package, where the relative import fails.
