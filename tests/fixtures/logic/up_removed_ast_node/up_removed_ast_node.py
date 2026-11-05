import ast


def is_text(node):
    return isinstance(node, ast.Str)


def is_bytes(node):
    return isinstance(node, ast.Bytes) or node is None


def is_test(node):
    if isinstance(node, ast.Str):
        return True
    return not isinstance(node, ast.Str)


def is_number(node):
    return isinstance(node, ast.Num)


def is_nested(nodes):
    return isinstance(nodes[0], ast.Str)


def is_in_tuple(node):
    return isinstance(node, (ast.Str, ast.Bytes))


def make_number():
    return ast.Num(n=1)


def make_text():
    return ast.Str(s="text")


def make_bytes():
    return ast.Bytes(s=b"raw")


def make_constant():
    return ast.NameConstant(value=None)


def make_positional():
    return ast.Num(1)


def make_extra(kind):
    return ast.Str(s="text", kind=kind)


def make_mixed():
    return ast.Str("text", s="text")


def make_commented():
    return ast.Num(  # the value
        n=1
    )


def make_ellipsis():
    return ast.Ellipsis()


def ellipsis_node(node):
    return node is ast.Ellipsis


def plain_reference():
    return ast.Constant


class Printer(ast.NodeVisitor):
    def visit_Num(self, node):
        self.count += 1

    def visit_Constant(self, node):
        self.count += 1


class Rewriter(ast.NodeTransformer):
    def visit_Str(self, node):
        return node


class Nested(Printer):
    def visit_Bytes(self, node):
        return node


class Unrelated:
    def visit_Num(self, node):
        return node


def visit_Num(node):
    return node
