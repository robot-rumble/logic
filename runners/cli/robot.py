import json
import sys

import stdlib
from stdlib import *


robot_states = {}

def _robot(state, unit, debug):
    robot_state = robot_states.setdefault(unit.id, {"d": Direction.East})
    print(unit.coords.x)
    if unit.coords.x == 17:
        robot_state["d"] = Direction.West
    elif unit.coords.x == 0:
        robot_state["d"] = Direction.East
    return move(robot_state["d"])


print('__rr_init:{"Ok":null}', flush=True)
for inp in sys.stdin:
    print(inp)
    inp = json.loads(inp)
    output = stdlib.__main(inp, scope=globals())
    sys.stdout.write("__rr_output:")
    json.dump(output, sys.stdout)
    print(flush=True)
