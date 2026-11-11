import ast


def is_text(node, isinstance=None):
    return isinstance(node, ast.Str)


def is_text_plain(node):
    return isinstance(node, ast.Str)
