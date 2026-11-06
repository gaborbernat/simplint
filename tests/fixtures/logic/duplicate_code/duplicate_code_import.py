import json
import os
import sys
import re
import math
import time


def config():
    return json.dumps(os.sep), sys.platform, re.compile("a"), math.pi, time.time()
