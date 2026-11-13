__all__ = []

import io
import os
import socket


class AppError(Exception):
    pass


class ConfigError(AppError):
    pass


def read(path):
    try:
        return open(path).read()
    except OSError:
        return ""
    except FileNotFoundError:
        return None


def parse(text):
    try:
        return int(text)
    except (ValueError, UnicodeDecodeError):
        return 0


def load(path):
    try:
        return read(path)
    except AppError:
        raise
    except (ConfigError, KeyError):
        return None


def close(stream):
    try:
        stream.close()
    except (OSError, io.UnsupportedOperation, socket.timeout):
        pass


def every_entry(path):
    try:
        return read(path)
    except LookupError:
        return None
    except OSError:
        return None
    except (KeyError, FileNotFoundError):
        return ""


def with_comment(path):
    try:
        return read(path)
    except (
        OSError,  # the base
        IsADirectoryError,
    ):
        return None


def unrelated(path):
    try:
        return os.stat(path)
    except FileNotFoundError:
        return None
    except OSError:
        return None
    except (ValueError, TypeError):
        return None


def same_class(path):
    try:
        return read(path)
    except (OSError, OSError):
        return None


def unknown(path, error):
    try:
        return read(path)
    except Exception:
        return None
    except error:
        return None


def single_base(path):
    try:
        return read(path)
    except LookupError:
        return None
    except (KeyError, IndexError):
        return ""
