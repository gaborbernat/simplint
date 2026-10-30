from package import other
from package.sub import sibling
from package.sub.deep import thing
from os import path
from . import already

VALUE = (other, sibling, thing, path, already)
