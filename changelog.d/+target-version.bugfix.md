`walrus-candidate` and `missing-final` do not run below Python 3.8, and `dependency-import-guard` treats a module as
standard library only when every supported version ships it, so guards around `distutils`, `ctypes`, `ssl`, `sqlite3` or
`tkinter` are no longer reported.
