#!/usr/bin/env python
import enum
import typing


def check_instance(val: typing.Any, cls: typing.Any, func_name: str):
    if not isinstance(val, cls):
        raise TypeError(f"{func_name} argument must be an instance of {cls.__name__}")


class Direction(enum.Enum):
    North = "North"
    South = "South"
    East = "East"
    West = "West"

    __repr__ = lambda self: self.__str__()

    @property
    def opposite(self) -> "Direction":
        return {
            Direction.East: Direction.West,
            Direction.West: Direction.East,
            Direction.South: Direction.North,
            Direction.North: Direction.South,
        }[self]

    @property
    def to_coords(self) -> "Coords":
        return {
            Direction.East: Coords(1, 0),
            Direction.West: Coords(-1, 0),
            Direction.South: Coords(0, 1),
            Direction.North: Coords(0, -1),
        }[self]

    @property
    def rotate_cw(self) -> "Direction":
        return {
            Direction.North: Direction.East,
            Direction.East: Direction.South,
            Direction.South: Direction.West,
            Direction.West: Direction.North,
        }[self]

    @property
    def rotate_ccw(self) -> "Direction":
        return {
            Direction.North: Direction.West,
            Direction.West: Direction.South,
            Direction.South: Direction.East,
            Direction.East: Direction.North,
        }[self]


class Coords(tuple):
    def __new__(cls, x: int, y: int) -> "Coords":
        check_instance(x, int, "Coords.__new__")
        check_instance(y, int, "Coords.__new__")
        self = super().__new__(cls, [x, y])
        return self

    def __repr__(self) -> str:
        return f"({self.x}, {self.y})"

    @property
    def x(self) -> int:
        return self[0]

    @property
    def y(self) -> int:
        return self[1]

    def distance_to(self, other: "Coords") -> float:
        import math
        check_instance(other, Coords, "Coords.distance_to")
        return math.sqrt((other.x - self.x) ** 2 + (other.y - self.y) ** 2)

    def walking_distance_to(self, other: "Coords") -> int:
        check_instance(other, Coords, "Coords.walking_distance_to")
        return abs(other.x - self.x) + abs(other.y - self.y)

    def coords_around(self) -> typing.List["Coords"]:
        return [self + direction for direction in Direction]

    def direction_to(self, other: "Coords") -> Direction:
        import math
        check_instance(other, Coords, "Coords.direction_to")
        diff = self - other
        angle = math.atan2(diff.y, diff.x)
        if abs(angle) <= math.pi / 4:
            return Direction.West
        elif abs(angle + math.pi / 2) <= math.pi / 4:
            return Direction.South
        elif abs(angle - math.pi / 2) <= math.pi / 4:
            return Direction.North
        else:
            return Direction.East

    def __add__(self, other: typing.Union["Coords", Direction]) -> "Coords":
        if isinstance(other, Coords):
            return Coords(self.x + other.x, self.y + other.y)
        elif isinstance(other, Direction):
            return Coords(self.x + other.to_coords.x, self.y + other.to_coords.y)
        else:
            raise TypeError('Coords.__add__ argument must be an instance of Coords or Direction')

    def __sub__(self, other: typing.Union["Coords", Direction]) -> "Coords":
        if isinstance(other, Coords):
            return Coords(self.x - other.x, self.y - other.y)
        elif isinstance(other, Direction):
            return Coords(self.x - other.to_coords.x, self.y - other.to_coords.y)
        else:
            raise TypeError('Coords.__sub__ argument must be an instance of Coords or Direction')

    def __mul__(self, n: int) -> "Coords":
        check_instance(n, int, "Coords.__mul__")
        return Coords(self.x * n, self.y * n)


class Team(enum.Enum):
    Red = "Red"
    Blue = "Blue"

    __repr__ = lambda self: self.__str__()

    @property
    def opposite(self) -> "Team":
        if self == Team.Red:
            return Team.Blue
        else:
            return Team.Red


class ObjType(enum.Enum):
    Unit = "Unit"
    Terrain = "Terrain"

    __repr__ = lambda self: self.__str__()


class Obj:
    def __init__(self, obj: dict) -> None:
        check_instance(obj, dict, "Coords.__init__")
        self.__data = obj

    def __repr__(self) -> str:
        if self.obj_type == ObjType.Unit:
            return f"<{self.obj_type} id={self.id} coords={self.coords} {self.team} health={self.health}>"
        else:
            return f"<{self.obj_type} id={self.id} coords={self.coords}>"

    @property
    def coords(self) -> Coords:
        return Coords(*self.__data["coords"])

    @property
    def id(self) -> str:
        return self.__data["id"]

    @property
    def obj_type(self) -> ObjType:
        return ObjType(self.__data["obj_type"])

    @property
    def team(self) -> typing.Optional[Team]:
        if self.obj_type == ObjType.Unit:
            return Team(self.__data["team"])
        else:
            return None

    @property
    def health(self) -> typing.Optional[int]:
        if self.obj_type == ObjType.Unit:
            return self.__data["health"]
        else:
            return None


class State:
    def __init__(self, state: dict) -> None:
        check_instance(state, dict, "State.__init__")
        self.__data = state

    @property
    def turn(self) -> int:
        return self.__data["turn"]

    @property
    def our_team(self) -> Team:
        return Team(self.__data["team"])

    @property
    def other_team(self) -> Team:
        return self.our_team.opposite

    def obj_by_id(self, id: str) -> typing.Optional[Obj]:
        check_instance(id, str, 'State.obj_by_id')
        try:
            return Obj(self.__data["objs"][id])
        except KeyError:
            return None

    def ids_by_team(self, team: Team) -> typing.List[str]:
        check_instance(team, Team, 'State.check_instance')
        return self.__data["teams"][team.value]

    def objs_by_team(self, team: Team) -> typing.List[Obj]:
        check_instance(team, Team, 'State.objs_by_team')
        return [self.obj_by_id(id) for id in self.ids_by_team(team)]

    def id_by_coords(self, coords: Coords) -> typing.Optional[str]:
        check_instance(coords, Coords, 'State.id_by_coords')
        try:
            return self.__data["grid"][coords.y][coords.x]
        except IndexError:
            return None

    def obj_by_coords(self, coords: Coords) -> typing.Optional[Obj]:
        check_instance(coords, Coords, 'State.obj_by_coords')
        id = self.id_by_coords(coords)
        if id:
            return self.obj_by_id(id)
        else:
            return None


class ActionType(enum.Enum):
    Attack = "Attack"
    Move = "Move"

    __repr__ = lambda self: self.__str__()


class Action:
    def __init__(self, type: ActionType, direction: Direction) -> None:
        check_instance(type, ActionType, 'Action.__init__')
        check_instance(direction, Direction, 'Action.__init__')
        self.type = type
        self.direction = direction

    def __repr__(self) -> str:
        return f"<{self.type} {self.direction}>"

    @staticmethod
    def move(direction: Direction) -> "Action":
        check_instance(direction, Direction, 'Action.move')
        return Action(ActionType.Move, direction)

    @staticmethod
    def attack(direction: Direction) -> "Action":
        check_instance(direction, Direction, 'Action.attack')
        return Action(ActionType.Attack, direction)


MAP_SIZE = 19


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
    import traceback
    tb_lines = list(traceback.TracebackException.from_exception(exc).format())
    # from docs: "The message indicating which exception occurred is always the last string in the output."
    return {
        "summary": tb_lines.pop().strip(),
        "details": "".join(tb_lines),
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
    import typing

    had_stdout, old_stdout = (
        (True, sys.stdout) if hasattr(sys, "stdout") else (False, None)
    )
    logbuf = sys.stdout = io.StringIO()

    state = State(state)
    try:
        robot = __validate_function("robot", 2, True)
        init_turn = __validate_function("init_turn", 1, False)
    except Exception as e:
        return {"Err": {"InitError": __format_err(e)}}

    if callable(init_turn):
        try:
            init_turn(state)
        except Exception as e:
            return {"Err": {"InitError": __format_err(e)}}

    robot_actions = {}
    debug_inspect_tables = {}
    debug_locate_queries = []

    for id in state.ids_by_team(state.our_team):
        global debug
        debug_inspect_table = {}

        class Debug:
            def inspect(self, key: str, val: typing.Any) -> None:
                check_instance(key, str, "Debug.inspect 'key'")
                debug_inspect_table[key] = str(val)

            def locate(self, unit: Obj) -> None:
                check_instance(unit, Obj, "Debug.locate")
                debug_locate_queries.append(unit.id)

        debug = Debug()

        try:
            action = robot(state, state.obj_by_id(id))
            if isinstance(action, Action):
                result = {
                    "Ok": {"type": action.type.value, "direction": action.direction.value}
                }
            elif action is None:
                result = {"Ok": None}
            else:
                raise TypeError("Robot must return an Action or None")
        except Exception as e:
            result = {"Err": __format_err(e)}

        robot_actions[id] = result
        if debug_inspect_table:
            debug_inspect_tables[id] = debug_inspect_table

    if had_stdout:
        sys.stdout = old_stdout
    else:
        del sys.stdout

    logbuf.seek(0)
    logs = logbuf.readlines()
    logbuf.close()

    del state

    return {
        "Ok": {
            "robot_actions": robot_actions,
            "logs": logs,
            "debug_inspect_tables": debug_inspect_tables,
            "debug_locate_queries": debug_locate_queries
        }
    }


del enum
del typing

if __name__ == "__main__":
    __builtins__.__dict__.update(globals())
    import sys, json, runpy
    try:
        json.use_serde_json()
    except AttributeError:
        pass

    module = sys.argv[1]
    module = runpy.run_path(module)
    print('__rr_init:{"Ok":null}', flush=True)
    for inp in sys.stdin:
        inp = json.loads(inp)
        output = __main(inp, scope=module)
        sys.stdout.write("__rr_output:")
        json.dump(output, sys.stdout)
        print(flush=True)
        output.clear()
