import warnings
from warnings import warn
import warnings as w


def connect(host):
    warnings.warn("connect() is deprecated, use open()", DeprecationWarning, stacklevel=2)
    return host


def documented(host):
    """Connect to a host."""
    warnings.warn("documented() is deprecated", category=DeprecationWarning)
    return host


class Client:
    """A client for the old protocol."""

    warnings.warn("Client is deprecated", DeprecationWarning)

    async def send(self, data):
        return data


def only_warning():
    warnings.warn("gone", DeprecationWarning)


def imported_name(host):
    warn("imported_name() is deprecated", DeprecationWarning)
    return host


def late_warning(host):
    host = host.strip()
    warnings.warn("late_warning() is deprecated", DeprecationWarning)
    return host


def other_category(host):
    warnings.warn("other_category() changes", FutureWarning)
    return host


def no_category(host):
    warnings.warn("no_category() is deprecated")
    return host


def no_message():
    warnings.warn(category=DeprecationWarning)


def not_a_warning(host):
    print("not_a_warning()", DeprecationWarning)
    return host


def assigned_result(host):
    result = warnings.warn("assigned_result()", DeprecationWarning)
    return result


def commented(host):
    warnings.warn(  # keep the old name
        "commented() is deprecated", DeprecationWarning
    )
    return host


def multiline(host):
    warnings.warn(
        "multiline() is deprecated, "
        "use another name",
        DeprecationWarning,
    )
    return host


def shared_line(host):
    warnings.warn("shared_line() is deprecated", DeprecationWarning); host = host.strip()
    return host


class Settings:
    @property
    def timeout(self):
        warnings.warn("timeout is deprecated", DeprecationWarning, stacklevel=2)
        return 1

    @classmethod
    def create(cls):
        warnings.warn(f"{cls.__name__}.create() is deprecated", DeprecationWarning)
        return cls()

    async def fetch(self):
        warnings.warn("fetch() is deprecated", DeprecationWarning)


def aliased_module(host):
    w.warn("aliased_module() is deprecated", DeprecationWarning)
    return host
