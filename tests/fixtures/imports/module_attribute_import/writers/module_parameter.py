from . import findings


def write(DbWriter: object) -> object:
    return findings.DbWriter()
