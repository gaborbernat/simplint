import logging
import re

logger = logging.getLogger(__name__)


def load(path):
    try:
        return open(path).read()
    except OSError as exc:
        if "denied" in str(exc):
            return None
        if exc.args[0] == 2:
            return ""
        if str(exc).startswith("timed"):
            return ""
        if re.search("gone", str(exc)):
            return ""
        match str(exc):
            case "missing":
                return None
        logger.error("failed: %s", str(exc))
        print(f"failed: {exc}")
        if exc.errno == 2:
            return None
        raise


def retry(call):
    try:
        return call()
    except TimeoutError as error:
        if error.message == "slow":
            return None
        return None
    except ValueError as error:
        if str(error) == "":
            raise
        return None
