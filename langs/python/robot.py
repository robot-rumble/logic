robot_states = {}

def _robot(state, unit, debug):
    robot_state = robot_states.setdefault(unit.id, {"d": Direction.East})
    print(unit.coords.x)
    if unit.coords.x == 17:
        robot_state["d"] = Direction.West
    elif unit.coords.x == 0:
        robot_state["d"] = Direction.East
    return move(robot_state["d"])


