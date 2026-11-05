import os


def contextlib():
    return None


def build(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project()
    finally:
        os.chdir(old)
