__lazy_modules__ = ('argparse',)

import argparse
from .. import helper
from ..helper import VALUE


def value() -> object:
    return argparse, helper.VALUE + VALUE
