def _pick(items: list[int]) -> int:
    """Return the largest item, or the first one on a tie.

    The order matters: callers rely on the first match winning, so a later equal item never
    replaces an earlier one.
    """
    return max(items)


def _handler(error: Exception) -> None:
    """Return the handler for the error, or None to let it propagate."""


class _Registry:
    """
    Keep plugins in registration order.

    Parameters
    ----------
    names
        the plugin names, in the order they were loaded
    """

    def lookup(self) -> None:  # the docstring shares no line with the header
        """Look up a plugin by name, falling back to the default plugin when none matches."""; print("x")


def _header() -> None: """Run the setup once per process, since a second run would reset state."""; print("y")


def _whole_header() -> None: """Run the setup once per process, since a second run would reset state."""


def _pragma() -> None:
    """Explain a suppression this line-length pragma was written for, and nothing else at all."""  # noqa: E501
    print("z")


def _commented() -> None:
    """Explain why the value is cached, which a comment after it also says."""  # cached
    print("w")


def _directive() -> None:
    """type: ignore the result here, since the stub lies about the return type."""
    print("v")


def _fields(name: str, count: int) -> None:
    """Store the entry under a name, keeping the first one stored when two entries share it.

    Args:
        name: the key the entry is stored under, compared case-insensitively against the others
        count (int): how many copies to keep
    See https://example.com/a/very/long/path/that/no/wrapping/can/split/across/two/lines/at/all/index.html
    """
    print(name, count)


def _empty() -> None:
    """"""
    print("u")


def _padded() -> None:
    """

    Retry once, as the first call after a restart always times out.

    """
    print("t")


def _joined() -> None:
    """Keep the comment after the semicolon, which would go with the docstring.""";  # why
    print("s")


def _unbreakable() -> None:
    """Convert the rule to its wire form, for example:

        '[{"sample_rate":1.0,"service":"my-service","resource":"*","name":"web.request","tags":{}}]'
    """
    print("r")


# fmt: off
def _frozen() -> None:
 """Run the setup once per process, since a second run would reset state."""
 print("q")
# fmt: on


def _split(text: str) -> list[str]:
    """Split text on \r\n, \r, or \n only, unlike str.splitlines."""
    return text.split("\n")


def _quoted() -> None:
    """Quote the source: https://example.com/a.html    # noqa: E501
    and keep going after it.
    """
    print("p")


def _thawed() -> None:
 """Run the setup once per process, since a second run would reset state."""
 print("o")


def _skipped() -> None:
 """Run the setup once per process, since a second run would reset state."""  # fmt: skip
 print("n")


class _Box:
    def first(self) -> None:
        if self:
            print("m")

    def second(self) -> None:
        """Keep the box sorted by weight, since the packer reads it from the heaviest."""
        print("l")
