The call-site rules, such as `unused-return-value` (SMP411), count a call made through a module that neither the run nor
the standard library of the newest Python provides, such as `log.set_threshold(...)` after `from distutils import log`
in a tree that shims `distutils.log`, as a call of the run's file of that last name, so a function whose value that call
uses is not reported. A standard-library module such as `json` never counts.
