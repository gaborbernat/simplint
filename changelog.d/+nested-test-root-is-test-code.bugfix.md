A nested project rooted at a `test` or `tests` directory, such as numpy's `tools/swig/test` with its own `setup.py`, has
its files treated as test code by every rule that reads test modules, as the directory name says, so `duplicate-code`
(SMP530) no longer compares them with the rest of the run.
