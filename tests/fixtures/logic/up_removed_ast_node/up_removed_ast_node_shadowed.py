import ast


def isinstance(node, kind):
    return False


def is_text(node):
    return isinstance(node, ast.Str)
