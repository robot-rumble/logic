import math
import enum

class Coords(tuple):
    def __new__(cls, x, y):
        self = super().__new__(cls, [x, y])
        return self

    def __repr__(self):
        return "({self.x}, {self.y})"

    @property
    def x(self):
        return self[0]

    @property
    def y(self):
        return self[1]

    def distance(self, other):
        return math.sqrt((other.x - self.x) ** 2 + (other.y - self.y) ** 2)

    def walking_distance(self, other):
        return abs(other.x - self.x) + abs(other.y - self.y)

    def coords_around(self, other):
        pass

    def toward(self, other):
        pass


class Team(enum.Enum):
    Red = "Red"
    Blue = "Blue"


class Obj:
    def __init__(self, obj):
        self.__data = obj

    @property
    def coords(self):
        return Coords(*self.__data["coords"])

    @property
    def id(self):
        return Coords(*self.__data["id"])


class State:
    def __init__(self, state_dict):
        self.__data = state_dict
        self.turn = state_dict["turn"]
        team = self.our_team = Team(state_dict["team"])
        if team == Team.Red:
            self.other_team = Team.Blue
        else:
            self.other_team = Team.Red

    def obj_by_id(self, id):
        return Obj(self.__data["objs"][id])

    def objs_by_team(self, team):
        return [self.obj_by_id(id) for id in self.ids_by_team(team)]

    def ids_by_team(self, team):
        if not isinstance(team, Team):
            raise TypeError("Team must be a Team")
        return self.__data["teams"][team.value]

    def obj_by_loc(self, coord):
        id = self.id_by_loc(coord)
        return id and self.obj_by_id(id)

    def id_by_loc(self, coord):
        xs = self.__data["grid"][coord.x]
        return xs and xs[coord.y]


class ActionType(enum.Enum):
    Attack = "Attack"
    Move = "Move"


class Direction(enum.Enum):
    North = "North"
    South = "South"
    East = "East"
    West = "West"


class Action:
    def __init__(self, type, direction):
        if not isinstance(type, ActionType):
            raise TypeError("Type must be an ActionType")
        if not isinstance(direction, Direction):
            raise TypeError("Direction must be a Direction")
        self.type = type
        self.direction = direction

    def __repr__(self):
        return f"<Action: {self.type} {self.direction}"


def move(direction):
    return Action(ActionType.Move, direction)


def attack(direction):
    return Action(ActionType.Attack, direction)

def __format_err(exc):
    return {
        # TODO(noah) get exception location
        "start": [0, 0],
        "end": [0, 0],
        "message": str(exc),
    }

def __main(state, scope=globals()):
    import sys, io 
    hadstdout, oldstdout = (True, sys.stdout) if hasattr(sys, "stdout") else (False, None)
    logbuf = sys.stdout = io.StringIO()

    state = State(state)
    _robot = scope.get("_robot")
    _init_turn = scope.get("_init_turn")
    try:
        if not callable(_robot):
            raise TypeError("You must define a '_robot' function")
        if _robot.__code__.co_argcount != 3:
            raise TypeError(
                "Your _robot function must accept 3 values: the current state, "
                "the details for the current unit, and a debug function."
            )
        if callable(_init_turn) and _init_turn.__code__.co_argcount != 1:
            raise TypeError(
                "If you choose to define an _init_turn function, it must accept 1 value: the current state."
            )
    except Exception as e:
        return {
            "robot_outputs": {"Err": {"RobotError": __format_err(e)}}
        }

    if callable(_init_turn):
        _init_turn(state)

    robot_outputs = {}
    for id in state.ids_by_team(state.our_team):
        debug_table = {}

        def debug(key, val):
            debug_table[key] = val

        try:
            action = _robot(state, state.obj_by_id(id), debug)
            if not isinstance(action, Action):
                raise TypeError("Your _robot function must return an Action")
        except Exception as e:
            result = {
                "Err": __format_err(e)
            }
        else:
            result = {
                "Ok": {"type": action.type.value, "direction": action.direction.value}
            }
        robot_outputs[id] = {
            "action": result,
            "debug_table": debug_table
        }

    if hadstdout:
        sys.stdout = oldstdout
    else:
        del sys.stdout

    logbuf.seek(0)
    logs = logbuf.readlines()
    logbuf.close()

    return {
        "robot_outputs": {"Ok": robot_outputs},
        "logs": logs
    }
