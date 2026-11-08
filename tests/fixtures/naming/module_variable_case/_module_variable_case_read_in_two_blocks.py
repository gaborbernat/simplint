import sys

if sys.platform == "win32":
    feeds = ["a.c"]
    libraries = []

if __name__ == "__main__":
    print(feeds, libraries)
