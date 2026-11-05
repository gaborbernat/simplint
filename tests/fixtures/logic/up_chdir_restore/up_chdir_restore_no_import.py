"""Build helpers."""


def build(target):
    import os

    old = os.getcwd()
    os.chdir(target)
    try:
        return compile_project()
    finally:
        os.chdir(old)
