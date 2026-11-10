import json


def main():
    return _load("{}")


def other():
    return 1


def _load(text, loads=json.loads):
    return loads(text)
