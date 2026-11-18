__lazy_modules__ = {"zipfile", "argparse"}

import argparse
import json
import zipfile


def parse() -> object:
    return argparse.ArgumentParser(), json, zipfile
