__lazy_modules__ = [
    "argparse",  # the CLI only
    "zipfile",
]

import argparse
import json
import zipfile


def parse() -> object:
    return argparse.ArgumentParser(), json, zipfile
