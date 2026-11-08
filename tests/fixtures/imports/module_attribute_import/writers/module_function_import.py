import sys


def __getattr__(name: str) -> object:
    import writers.findings as findings

    return findings


if sys.platform == "darwin":
    from . import findings

    writer = findings.DbWriter()
