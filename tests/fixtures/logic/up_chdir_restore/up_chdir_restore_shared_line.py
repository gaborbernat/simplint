import os


def build(target):
    count = 0; old = os.getcwd()
    os.chdir(target)
    try:
        run()
    finally:
        os.chdir(old)
    return count
