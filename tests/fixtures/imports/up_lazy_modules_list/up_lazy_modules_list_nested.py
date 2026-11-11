__lazy_modules__ = ["json", "pathlib"]

import json
from pathlib import Path

if __name__ == "__main__":
    import json as nested_json

print(json.dumps([1]), Path("demo"), nested_json)
