import logging
from logging import warning

logger = logging.getLogger(__name__)


def greet(name: str, count: int) -> None:
    logger.info("Hello, " + name)
    logger.info(msg="Hello, " + name)
    logging.log(logging.INFO, "Hello, " + name)
    logger.info(("Hello, " + name) * 2)
    warning("Hello, " + name)
    print("Hello, " + name)


def label() -> str:
    return (
        "Hello, "
        + "world"
    )
