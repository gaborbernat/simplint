import sys

MINIMUM = (3, 6)
EXIT_CODE = 79

if __name__ == "__main__" and sys.version_info[:2] < MINIMUM:
    sys.stderr.write("unsupported")
    raise SystemExit(EXIT_CODE)

import json
