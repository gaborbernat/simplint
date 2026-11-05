from . import findings


def make[DbWriter](kind: type[DbWriter]) -> object:
    return findings.DbWriter()
