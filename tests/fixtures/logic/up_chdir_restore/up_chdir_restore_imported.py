import contextlib
import os


def build(target):
    old = os.getcwd()
    os.chdir(target)
    try:
        return contextlib.nullcontext(compile_project())
    finally:
        os.chdir(old)
