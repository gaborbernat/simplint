def main():
    return _load("{}")


def other():
    return 1


import json


def _load(text, loads=json.loads):
    return loads(text)
