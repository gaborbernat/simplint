lazy import gzip
lazy from bz2 import decompress


def unpack(data):
    return decompress(gzip.decompress(data))
