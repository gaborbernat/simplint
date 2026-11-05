__lazy_modules__ = ["json", "pathlib"]

import json, os
from pathlib import Path
from . import sibling
from json import *

print(json.dumps([1]), os.sep, Path("demo"), sibling)
