`dependency-import-guard` (SMP040) no longer reports `from` imports, which can fail on a module that lacks the name on
some platform or version, such as `from os import geteuid`.
