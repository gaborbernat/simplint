import sys

LIMIT = 3

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib
