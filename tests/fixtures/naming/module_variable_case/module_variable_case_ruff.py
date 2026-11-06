import os
from collections import namedtuple
from typing import NamedTuple

myValue = 1
MYVALUE = 2
print(myValue, MYVALUE)

limitValue = 3
print(limitValue)

_privateValue = 1
_PRIVATEVALUE = 2
print(_privateValue, _PRIVATEVALUE)

__dunderValue = 1
__DUNDERVALUE = 2
print(__dunderValue, __DUNDERVALUE)

maxDiff = None
MAXDIFF = 3
print(maxDiff, MAXDIFF)

pointTuple = namedtuple("pointTuple", "x y")
POINTTUPLE = 1
print(pointTuple, POINTTUPLE)

typedTuple = NamedTuple("typedTuple", [("x", int)])
TYPEDTUPLE = 1
print(typedTuple, TYPEDTUPLE)

notTuple = (1, 2)
NOTTUPLE = 1
print(notTuple, NOTTUPLE)

if os.sep:
    nestedTuple = namedtuple("nestedTuple", "x y")
elif os.altsep:
    elifTuple = namedtuple("elifTuple", "x y")
else:
    elseTuple = namedtuple("elseTuple", "x y")
NESTEDTUPLE = 1
ELIFTUPLE = 2
ELSETUPLE = 3
print(nestedTuple, elifTuple, elseTuple, NESTEDTUPLE, ELIFTUPLE, ELSETUPLE)

for _ in range(1):
    forTuple = namedtuple("forTuple", "x")
FORTUPLE = 1
while os.sep:
    whileTuple = namedtuple("whileTuple", "x")
    break
WHILETUPLE = 1
with open(os.devnull) as handle:
    withTuple = namedtuple("withTuple", "x")
WITHTUPLE = 1
try:
    tryTuple = namedtuple("tryTuple", "x")
except ValueError:
    handlerTuple = namedtuple("handlerTuple", "x")
else:
    elseTryTuple = namedtuple("elseTryTuple", "x")
finally:
    finallyTuple = namedtuple("finallyTuple", "x")
match os.sep:
    case "/":
        caseTuple = namedtuple("caseTuple", "x")

(tupleA, tupleB) = namedtuple("tupleA", "x y")
[listTuple] = namedtuple("listTuple", "x")
starredTuple, *restTuple = namedtuple("starredTuple", "x y")
os.namedAttr = namedtuple("namedAttr", "x")

print(handle, FORTUPLE, WHILETUPLE, WITHTUPLE, tupleA, tupleB, listTuple, starredTuple, restTuple, caseTuple)


def set_global() -> None:
    global sharedValue
    sharedValue = 1


SHAREDVALUE = 2
print(SHAREDVALUE, set_global)

stringName = 1
print("stringName")


def shadow(stringName: int) -> int:
    return stringName
