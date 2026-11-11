import os


class K:
    contextlib = 1
    old = os.getcwd()
    os.chdir("..")
    try:
        pass
    finally:
        os.chdir(old)


def outer(contextlib):
    def run(target):
        old = os.getcwd()
        os.chdir(target)
        try:
            return os.listdir()
        finally:
            os.chdir(old)

    return run


def run(target):
    def helper(contextlib):
        return contextlib

    old = os.getcwd()
    os.chdir(target)
    try:
        return os.listdir()
    finally:
        os.chdir(old)
