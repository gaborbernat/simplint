def literal_holds_no_role() -> None:
    """Show the literal form.

    Write ``open :class:Path`` for the literal, as the reference is code.
    """


def quoted_bare_role() -> None:
    """Open the file, as ":class:Path" does."""


def role_after_interpreted_text() -> None:
    """Open the file: `open`:class:Path does."""


def role_in_a_comment() -> None:
    """Notes.

    .. :class:Path is a comment.
    """


def escaped_colon() -> None:
    """Write a\\:class:Path to keep it plain."""
