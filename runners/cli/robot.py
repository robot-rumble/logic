import json, sys
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


for inp in sys.stdin:
    output = stdlib.__main(inp, scope=globals())
    print("__rr_output:", output, flush=True)
