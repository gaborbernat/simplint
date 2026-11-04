"""Module summary.

The cache keeps one entry per manifest, keyed by path and modification time, because parsing a manifest dominates
the start-up time of large projects. An entry written by one interpreter is not valid for another, since the parsed
form depends on the version of the grammar, so the cache directory is named after the interpreter. Entries older
than thirty days are removed on the next run, and a corrupt entry is discarded rather than reported, because the
file it came from is still on disk and can be parsed again. Writes go to a temporary file first. Readers never lock,
since a replaced file is either the old one or the new one.
"""


class Cache:
    """Short class docstring."""

    def load(self) -> None:
        """Load the manifest.

        The cache keeps one entry per manifest, keyed by path and modification time, because parsing a
        manifest dominates the start-up time of large projects. An entry written by one interpreter is not
        valid for another, since the parsed form depends on the version of the grammar, so the cache
        directory is named after the interpreter. Entries older than thirty days are removed on the next
        run, and a corrupt entry is discarded rather than reported, because the file it came from is still
        on disk and can be parsed again. Writes go to a temporary file first. Readers never lock, since a
        replaced file is either the old one or the new one.

        A second paragraph that stays short.
        """


def outer() -> None:
    def inner() -> None:
        """The cache keeps one entry per manifest, keyed by path and modification time, because parsing a
        manifest dominates the start-up time of large projects. An entry written by one interpreter is not
        valid for another, since the parsed form depends on the version of the grammar, so the cache
        directory is named after the interpreter. Entries older than thirty days are removed on the next
        run, and a corrupt entry is discarded rather than reported, because the file it came from is still
        on disk and can be parsed again. Writes go to a temporary file first. Readers never lock, since a
        replaced file is either the old one or the new one."""


def split() -> None:
    (
        "Short first part. "
        "Short second part."
    )


def doctest() -> None:
    """Run it.

    >>> outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer()
    ... outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer(); outer()
    """
