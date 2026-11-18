__lazy_modules__ = ["argparse"]
__lazy_modules__ += ["json"]

import argparse
import json
import zipfile


def parse() -> object:
    return argparse.ArgumentParser(), json, zipfile
