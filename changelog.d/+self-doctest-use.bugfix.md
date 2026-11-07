`unused-definition` (SMP409) no longer reports a class or function that a doctest of its module uses without importing
it, since a doctest runs in the globals of its module.
