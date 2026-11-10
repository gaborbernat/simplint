import argparse
from argparse import ArgumentParser

sole = argparse.ArgumentParser(suggest_on_error=True)
trailing = argparse.ArgumentParser(prog="tool", suggest_on_error=True,)
leading = argparse.ArgumentParser(suggest_on_error=True, prog="tool")
positional = argparse.ArgumentParser("tool", suggest_on_error=True)
multiline = argparse.ArgumentParser(
    suggest_on_error=True,
    prog="tool",
)
imported = ArgumentParser(prog="tool", suggest_on_error=True)
disabled = argparse.ArgumentParser(prog="tool", suggest_on_error=False)
dynamic = argparse.ArgumentParser(prog="tool", suggest_on_error=flag)
parenthesized = argparse.ArgumentParser(("prog"), suggest_on_error=True)


def build_other(parser_class):
    return parser_class(suggest_on_error=True)
