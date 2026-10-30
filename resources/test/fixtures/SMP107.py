import argparse


def options(parser: argparse.ArgumentParser) -> dict[str, str]:
    return vars(parser.parse_args())
