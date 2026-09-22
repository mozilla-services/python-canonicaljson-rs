from typing import Any, TextIO


__version__: str


def dump(obj: Any, fp: TextIO) -> None: ...
def dumps(obj: Any) -> str: ...
