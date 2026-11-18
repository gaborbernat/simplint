__lazy_modules__ = [
    # the CLI only
    "argparse",
]

import argparse
import json


def parse() -> object:
    return argparse.ArgumentParser(), json
