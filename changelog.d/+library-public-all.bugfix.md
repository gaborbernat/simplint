`missing-from-all` (SMP401) keeps reporting a name that other modules import but `__all__` leaves out in a public module
of a library, and offers no fix there: listing it widens the library's API and what `import *` binds.
