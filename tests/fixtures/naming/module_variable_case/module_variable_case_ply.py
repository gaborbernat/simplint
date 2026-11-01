from ply import lex

tokens = ("NUMBER", "PLUS")
literals = ["(", ")"]
t_PLUS = r"\+"
t_ignore = " \t"
word_values = {"one": 1}


def t_NUMBER(token):
    token.value = word_values.get(token.value, token.value)
    return token


def t_error(token):
    token.lexer.skip(1)


lexer = lex.lex()
