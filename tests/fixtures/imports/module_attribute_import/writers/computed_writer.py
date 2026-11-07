import sys

from . import computed_quotas

setattr(computed_quotas, sys.argv[1], 4)
