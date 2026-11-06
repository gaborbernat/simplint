import gzip
import bz2 as bzip
import os, lzma
from zlib import compress
from gzip import GzipFile
from compression.lzma import LZMAFile
from . import gzip as local_gzip
from .bz2 import BZ2File
import json


def pack(path, data):
    with gzip.open(path, "wb") as handle:
        handle.write(data)
    return bzip.BZ2File(path), GzipFile, LZMAFile, compress, os, json, local_gzip, BZ2File


def in_function():
    import lzma
    return lzma


try:
    import zlib
except ImportError:
    zlib = None


def internals():
    import zlib as private_zlib
    from gzip import READ
    from bz2 import *
    return private_zlib._ZlibDecompressor, READ, BZ2File
