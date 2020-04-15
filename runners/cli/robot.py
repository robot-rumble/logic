import json, sys
import stdlib
from stdlib import *


def robot(state, unit, debug):
    return move(Direction.East)


for inp in sys.stdin:
    output = stdlib._main(inp, scope=globals())
    print("__rr_output:", output, flush=True)
