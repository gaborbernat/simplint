import sys

if sys.platform == "darwin":
    from . import findings

    writer = findings.DbWriter()
else:
    import writers.findings as findings

    limit = findings.LIMIT
