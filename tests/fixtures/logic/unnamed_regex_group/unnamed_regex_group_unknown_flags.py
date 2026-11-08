import os
import re

FLAGS = re.VERBOSE


def pair(text):
    return re.match(r"(a) # (b)", text, FLAGS)


def ignore_case(text):
    return re.match(r"(a) # (b)", text, re.IGNORECASE)


def named_verbose(text):
    return re.match(r"(a) # (b)", text, re.RegexFlag.VERBOSE)


def unknown_flag(text):
    return re.match(r"(a) # (b)", text, re.FOO_FLAG)


def inline_verbose(text, flags):
    return re.match(r"(?x) (a)(b) # tail", text, flags)


def other_module_flag(text):
    return re.match(r"(a) # (b)", text, os.O_RDONLY)
