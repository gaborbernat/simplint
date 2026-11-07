`missing-from-all` (SMP401) no longer lists `annotations` from `from __future__ import annotations` as a name the other
modules import from a bare module entry point, such as a pytest plugin. A `__future__` import binds no name its users
read.
