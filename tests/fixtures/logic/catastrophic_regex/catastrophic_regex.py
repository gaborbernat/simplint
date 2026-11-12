import re

NESTED = re.compile(r"(a+)+$")
WORDS = re.compile(r"^(\w+\s?)*$")
ALTERNATIVE = re.compile(r"(x|y+)+z")
BOUNDED_MINIMUM = re.compile(r"(?:a+){2,}b")
NAMED = re.compile(r"(?P<word>[a-z]+)*!")
LAZY = re.compile(r"((\d)*?)*?x")
INNER = re.compile(r"x(?:y(a+)+)z")
VERBOSE = re.compile(
    r"""
    (\w+  # word
     \s?)*
    $
    """,
    re.VERBOSE,
)
INLINE_VERBOSE = re.compile(r"(?x) ( a + ) + $")
SEPARATED = re.compile(r"(\d+\.)+\d+")
TWO_REPEATS = re.compile(r"(a+b+)+")
POSSESSIVE = re.compile(r"(a+)++")
INNER_POSSESSIVE = re.compile(r"(a++)+")
ATOMIC = re.compile(r"(?>a+)+")
BOUNDED = re.compile(r"(a+){3}")
OPTIONAL = re.compile(r"(a+)?")
CLASS = re.compile(r"([]a-z\]]+)c")
NEGATED_CLASS = re.compile(r"([^]a]+)c")
LITERAL_BRACE = re.compile(r"(a+){x}")
EMPTY_BRACE = re.compile(r"a{,}")
UPPER_BOUND = re.compile(r"(a+){,5}")
LOOKAHEAD = re.compile(r"(?=a+)+")
LOOKBEHIND = re.compile(r"((?<=a)b)+")
COMMENT = re.compile(r"(?#note)a+")
BACKREFERENCE = re.compile(r"(?P<x>a)(?P=x)+")
CONDITIONAL = re.compile(r"(a)?(?(1)(b+)+|c)")
SCOPED_VERBOSE = re.compile(r"(?x:(a+)+)")
FLAGS = re.compile(r"(?i)(a+)+")
SCOPED_FLAGS = re.compile(r"(?i:(a+)+)")
BAD_FLAG = re.compile(r"(?Z)")
UNBALANCED = re.compile(r"(a+")
EXTRA_CLOSE = re.compile(r"a)")
OPTIONAL_INNER = re.compile(r"((a*)?)+b")
