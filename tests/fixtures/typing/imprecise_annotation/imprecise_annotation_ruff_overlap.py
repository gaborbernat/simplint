from typing import Annotated, Any, Optional, Union


def read(value: Optional[Any]) -> None:
    print(value)


def merge(value: Union[int, Any], *args: Any) -> None:
    print(value, args)


def tag(value: Annotated[Any, "meta"]) -> list[Any]:
    return [value]
