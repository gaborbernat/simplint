import sys

from star_conditional_source import *

print(conditional_base())
if sys.platform == "win32":
    print(conditional_extra())
