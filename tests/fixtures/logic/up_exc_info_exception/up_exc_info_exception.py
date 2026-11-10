import sys
import sys as system
import traceback
from sys import exc_info


def handle(run, other):
    try:
        run()
    except Exception:
        first = sys.exc_info()[1]
        second = system.exc_info()[1]
        traceback.print_exception(*sys.exc_info())
        traceback.print_exception(*system.exc_info(), limit=2)
        kind = sys.exc_info()[0]
        trace = sys.exc_info()[2]
        arg = sys.exc_info(1)[1]
        third = exc_info()[1]
        unknown = other.exc_info()[1]
        indexed = other[1]
        unpacked = traceback.print_exception(*other())
        plain = traceback.print_exception(*sys.exc_info(1))
        counted = traceback.print_exception(sys.exc_info()[1], *other)
        return first, second, kind, trace, arg, third, unknown, indexed, unpacked, plain, counted


def commented(run):
    try:
        run()
    except Exception:
        kept = sys.exc_info(  # keep the call
        )[1]
        traceback.print_exception(*sys.exc_info(  # keep the call
        ))
        return kept
