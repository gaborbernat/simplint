import os


def too_wide(resolved: object, root: str) -> bool:
    resolved_str = str(resolved)
    if resolved_str != root and not resolved_str.startswith(root + os.sep + "package"):
        return False
    return True


def too_wide_whatever_the_layout(resolved: object, root: str) -> bool:
    resolved_str = str(
        resolved
    )
    if (
        resolved_str != root
        and not resolved_str.startswith(root + os.sep + "package")
    ):
        return False
    return True


def fits_once_joined(resolved: object, root: str) -> bool:
    resolved_str = str(
        resolved
    )
    if (
        resolved_str != root
        and not resolved_str.startswith(root)
    ):
        return False
    return True


def inlined_too_wide(resolved: object, root: str) -> bool:
    joined = os.path.join(str(resolved), "package", "module", "submodule", "leaf")
    if not joined.startswith(root + os.sep):
        return False
    return True


def comment_in_test(resolved: object, root: str) -> bool:
    resolved_str = str(resolved)
    if (
        resolved_str != root  # the root itself
        and not resolved_str.startswith(root)
    ):
        return False
    return True


def multiline_string(resolved: object) -> bool:
    text = """
    one
    """
    if text != str(resolved):
        return False
    return True


def fits_without_the_parentheses(resolved: object, root: str) -> bool:
    resolved_str = str(resolved)
    if (
        resolved_str != root
        and not resolved_str.startswith(root, 1)
    ):
        return False
    return True
