def build_cli() -> int:
    return 1


_build_cli = build_cli
alias = build_cli
limit = build_cli()
_last = limit

import json
from json import JSONDecoder, dumps
from string import ascii_letters


class _Codec:
    def encode(self, value: object) -> str:
        return dumps(value)


_codec = _Codec()
encode = _codec.encode
to_json = dumps
loads = json.loads
decoder = JSONDecoder
letters = ascii_letters
indent = json.INDENT
size = _codec.size
_twice = _Codec()
_twice = _Codec()
twice_encode = _twice.encode
_made = build_cli()
made_encode = _made.encode
scanner = json.decoder.scanstring
_table = {"a": 1}
lookup = _table.get
