import sys

try:
    from _star_import_speedups import *
except ImportError:
    from star_listed_source import *
else:
    pass
finally:
    pass
if sys.platform == "win32":
    pass
elif sys.platform == "linux":
    from star_unlisted_source import *

print(listed_area(1), unlisted_double(2))
