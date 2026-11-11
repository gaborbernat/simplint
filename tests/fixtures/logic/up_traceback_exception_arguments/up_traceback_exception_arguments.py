import sys
import traceback
import traceback as tb_module
from traceback import print_exception


def report(run, saved, other):
    try:
        run()
    except Exception as error:
        traceback.print_exception(type(error), error, error.__traceback__)
        traceback.print_exception(error.__class__, error, error.__traceback__, limit=2, chain=False)
        traceback.format_exception(type(error), error, error.__traceback__)
        traceback.format_exception_only(type(error), error)
        tb_module.format_exception(type(error), error, error.__traceback__)
        print_exception(type(error), error, error.__traceback__, file=sys.stderr)
        traceback.format_exception(error.__class__, error, error.__traceback__, limit=1)
        traceback.format_exception(type(error), error, saved)
        traceback.format_exception(type(error), other, error.__traceback__)
        traceback.format_exception_only(type(error), other)
        traceback.format_exception(type(error), error)
        traceback.format_exception(type(error), error, error.__traceback__, None)
        traceback.format_exception(type(run()), error, error.__traceback__)
        traceback.format_exception(type(error.args), error, error.__traceback__)
        traceback.format_exception(type(error, 1), error, error.__traceback__)
        traceback.format_exception(type(error), error, error.__traceback__, *other)
        traceback.format_exception(*other)
        traceback.format_exception(error)
        traceback.format_exception(error.a.__class__, error, error.__traceback__)
        traceback.format_exception(error.__class__, error, error.a.__traceback__)
        traceback.format_exception(saved.__class__, error, saved.__traceback__)
        traceback.print_exception(type(error), error)
        traceback.format_exception_only(type(error), error, error.__traceback__)
        traceback.format_exception_only(type(error))
        other_module.format_exception(type(error), error, error.__traceback__)
        traceback.print_exception(run(), error, error.__traceback__)
        traceback.format_exception(run)
        run()
        print(type(error), error, error.__traceback__)


def shadowed(error):
    type = str
    try:
        raise ValueError
    except ValueError as error:
        return traceback.format_exception(type(error), error, error.__traceback__)


def commented(run):
    try:
        run()
    except Exception as error:
        traceback.print_exception(type(error),  # keep the type
                                  error, error.__traceback__)


def parenthesized(run):
    try:
        run()
    except Exception as error:
        traceback.print_exception((type(error)), error, error.__traceback__)
        traceback.format_exception((type(error)), error, (error.__traceback__))
        return traceback.format_exception_only(type(error), (error))
