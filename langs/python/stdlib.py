#!/usr/bin/env python
import enum
import math


class Direction(enum.Enum):
    North = "North"
    South = "South"
    East = "East"
    West = "West"

    @property
    def opposite(self):
        return {
            Direction.East: Direction.West,
            Direction.West: Direction.East,
            Direction.South: Direction.North,
            Direction.North: Direction.South,
        }[self]

    @property
    def to_coords(self):
        return {
            Direction.East: Coords(1, 0),
            Direction.West: Coords(-1, 0),
            Direction.South: Coords(0, 1),
            Direction.North: Coords(0, -1),
        }[self]


class ActionType(enum.Enum):
    Attack = "Attack"
    Move = "Move"


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

    def coords_around(self):
        [self + direction for direction in Direction]

    def towards(self, other):
        diff = other - self
        angle = math.atan2(diff.y, diff.x)
        if abs(angle) > math.pi / 4:
            if diff.y > 0:
                return Direction.North
            else:
                return Direction.South
        else:
            if diff.x > 0:
                return Direction.East
            else:
                return Direction.West

    def __add__(self, other):
        return Coords(self.x + other.x, self.y + other.y)

    def __sub__(self, other):
        return Coords(self.x - other.x, self.y - other.y)

    def __mul__(self, n):
        return Coords(self.x * n, self.y * n)


class Team(enum.Enum):
    Red = "Red"
    Blue = "Blue"

    @property
    def opposite(self):
        if self == Team.Red:
            return Team.Blue
        else:
            return Team.Red


class ObjType(enum.Enum):
    Unit = "Unit"
    Terrain = "Terrain"


class Obj:
    def __init__(self, obj):
        self.__data = obj

    @property
    def coords(self):
        return Coords(*self.__data["coords"])

    @property
    def id(self):
        return self.__data["id"]

    @property
    def obj_type(self):
        return ObjType(self.__data["obj_type"])

    @property
    def team(self):
        if self.obj_type == ObjType.Unit:
            return Team(self.__data["team"])

    @property
    def health(self):
        if self.obj_type == ObjType.Unit:
            return self.__data["health"]


class State:
    def __init__(self, state):
        self.__data = state

    @property
    def turn_num(self):
        return self.__data["turn_num"]

    @property
    def our_team(self):
        return Team(self.__data["team"])

    @property
    def other_team(self):
        return self.our_team.opposite()

    def ids_by_team(self, team):
        return self.__data["teams"][team.value]

    def obj_by_id(self, id):
        return Obj(self.__data["objs"][id])

    def objs_by_team(self, team):
        return [self.obj_by_id(id) for id in self.ids_by_team(team)]

    def id_by_coords(self, coords):
        return self.__data["grid"][coords.x][coords.y]

    def obj_by_coords(self, coords):
        return self.obj_by_id(self.id_by_coords(coords))


class Action:
    def __init__(self, type, direction):
        self.type = type
        self.direction = direction

    def __repr__(self):
        return f"<Action: {self.type} {self.direction}>"

    @staticmethod
    def move(direction):
        return Action(ActionType.Move, direction)

    @staticmethod
    def attack(direction):
        return Action(ActionType.Attack, direction)


def __format_err(exc):
    loc = None
    tb = exc.__traceback__
    while tb:
        if tb.tb_frame.f_code.co_filename == "<robot>":
            loc = {
                "start": (tb.tb_lineno, None),
                "end": None,
            }
        tb = tb.tb_next
    return {
        "message": str(exc),
        "loc": loc,
    }


def __main(state, scope=globals()):
    def __validate_function(name, argcount, mandatory):
        f = scope.get(name)
        if not callable(f):
            if mandatory:
                raise TypeError(f"You must define a '{name}' function")
        else:
            if f.__code__.co_argcount != argcount:
                raise TypeError(
                    f"Your {name} function must accept {argcount} arguments"
                )
        return f

    import sys, io

    had_stdout, old_stdout = (True, sys.stdout) if hasattr(sys, "stdout") else (False, None)
    logbuf = sys.stdout = io.StringIO()

    state = State(state)
    try:
        _robot = __validate_function("_robot", 3, True)
        _init_turn = __validate_function("_init_turn", 1, False)
    except Exception as e:
        return {"robot_outputs": {"Err": {"InitError": __format_err(e)}}}

    if callable(_init_turn):
        _init_turn(state)

    robot_outputs = {}
    for id in state.ids_by_team(state.our_team):
        debug_table = {}

        def debug(key, val):
            debug_table[key] = str(val)

        try:
            action = _robot(state, state.obj_by_id(id), debug)
            if not isinstance(action, Action):
                raise TypeError("Your _robot function must return an Action")
        except Exception as e:
            result = {"Err": __format_err(e)}
        else:
            result = {
                "Ok": {"type": action.type.value, "direction": action.direction.value}
            }
        robot_outputs[id] = {"action": result, "debug_table": debug_table}

    if had_stdout:
        sys.stdout = old_stdout
    else:
        del sys.stdout

    logbuf.seek(0)
    logs = logbuf.readlines()
    logbuf.close()

    return {"robot_outputs": {"Ok": robot_outputs}, "logs": logs}


del enum

if __name__ == '__main__':
    __builtins__.__dict__.update(globals())
    import sys, json, runpy

    module = sys.argv[1]
    module = runpy.run_path(module)
    print('__rr_init:{"Ok":null}', flush=True)
    for inp in sys.stdin:
        inp = json.loads(inp)
        output = __main(inp, scope=module)
        sys.stdout.write("__rr_output:")
        json.dump(output, sys.stdout)
        print(flush=True)
