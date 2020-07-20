import pytest
import datetime
import io

import canonicaljson


FIXTURES = [
    (None, "null"),
    ({"b": 2, "a": 1}, '{"a":1,"b":2}'),
    ({None: 2, 42: "", True: 1, False: 2}, '{"42":"","false":2,"null":2,"true":1}'),
    (["b", 2, 1], '["b",2,1]'),
    (("on", "off"), '["on","off"]'),
    (1, "1"),
    (3.14, "3.14"),
    (False, "false"),
    (True, "true"),
    ("s", '"s"'),
    ("é", '"\\u00e9"'),
    (10.0**21, '1e+21'),
]

@pytest.mark.parametrize("value,expected", FIXTURES)
def test_dumps(value, expected):
    assert canonicaljson.dumps(value) == expected


@pytest.mark.parametrize("value,expected", FIXTURES)
def test_dump(value, expected):
    s = io.StringIO()
    canonicaljson.dump(value, s)
    assert s.getvalue() == expected


class Unserializable:
    def __str__(self):
        raise ValueError("boom!")


@pytest.mark.parametrize("value,msg", [
    (datetime.datetime.now(), "Invalid type: datetime.datetime"),
    ({Unserializable(): "a"}, "Dictionary key is not serializable: Unserializable"),
    ({"a": datetime.datetime.now()}, "Invalid type: datetime.datetime")
])
def test_unserializable(value, msg):
    with pytest.raises(TypeError) as excinfo:
        canonicaljson.dumps(value)
    assert msg in repr(excinfo.value)
