def send(data: str, flush: bool = False) -> str:
    return data.strip() if flush else data


def retry(count: int, delay: float = 1.0) -> float:
    return count * delay


def connect(host: str, timeout: int = 30) -> str:
    return f"{host}:{timeout}"


def dispatch(payload: str, verbose: bool = False) -> str:
    return payload if verbose else payload.strip()


def orphan(value: int, scale: int = 1) -> int:
    return value * scale


def main(argv: list[str] | None = None) -> int:
    return len(argv or [])


if __name__ == "__main__":
    main()
