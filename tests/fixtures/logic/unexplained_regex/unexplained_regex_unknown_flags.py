import re

FLAGS = re.VERBOSE

DATE = re.compile(
    r"""
    (\d+)  # year
    -
    (\d+)  # month
    -
    (\d+)  # day
    (x)  # tail
    """,
    FLAGS,
)
TIME = re.compile(r"(\d+)-(\d+)-(\d+)-(\d+)", re.IGNORECASE | re.MULTILINE)
SKIPPED = re.compile(r"(\d+)-(\d+)-(\d+)-(\d+)", re.IGNORECASE | FLAGS)
