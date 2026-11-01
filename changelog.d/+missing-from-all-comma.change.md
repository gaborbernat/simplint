`missing-from-all` writes a trailing comma when it creates `__all__`, and creating `__all__` is now an unsafe fix, since
it narrows what `from module import *` exports.
