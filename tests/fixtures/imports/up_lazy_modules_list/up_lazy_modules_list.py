__lazy_modules__ = ["json", "pathlib"]

import json
import pathlib
from os import path

print(json.dumps([1]), pathlib.Path("demo").name, path.sep)

for name in ("a",):
    print(name)
