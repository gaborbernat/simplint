import ply.yacc as yacc

__all__ = ["build"]


def p_expression(production):
    production[0] = production[1]


def p_error(production):
    raise SyntaxError(production)


def evaluate(text):
    return text


def build():
    return yacc.yacc()
