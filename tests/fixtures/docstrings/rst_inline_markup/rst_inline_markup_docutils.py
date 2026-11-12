def plurals() -> None:
    """Lists in the docs.

    Python ``list``s are lists.
    """


def comments() -> None:
    """Comments and raw content are not prose.

    .. a comment with ``x
       continued

    .. raw:: html

       <b>``x</b>

    Then a paragraph with ``open.
    """


def empty_comment() -> None:
    """A block quote after an empty comment is prose.

    ..

        A ``quoted block.
    """


def quoted_starts() -> None:
    """Starts that docutils reads as text.

    Quoted "``" is text, and ``a `` b`` is closed.
    """


def literal_block_after_paragraph() -> None:
    """Code under a paragraph of several lines.

    An example of
    code::
        x = ``unclosed
    """


def bullets() -> None:
    """Items are paragraphs of their own.

    - first ``open
    - second ``closed``
    """


def fences_and_mismatches() -> None:
    """Fences and mismatched backticks.

    ```python
    open_file("a")
    ```

    Call `Path`` here, and ``open` there.
    """
