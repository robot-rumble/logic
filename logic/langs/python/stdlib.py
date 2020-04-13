import math
import enum

from random import randrange


class Coords(tuple):
    def __new__(cls, x, y):
        self = super().__new__(cls, [x, y])
        return self

    def __repr__(self):
        return "Coords(x={self.x}, y={self.y})"

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
    def __init__(self, objdata):
        self.__obj = objdata # [basic, details]

    @property
    def coords(self):
        return Coords(*self.__obj[0]["coords"])


class State:
    def __init__(self, statedict):
        self.__state = statedict
        self.turn = statedict["turn"]
        team = self.our_team = Team(statedict["team"])
        if team == Team.Red:
            self.other_team = Team.Blue
        else:
            self.other_team = Team.Red

    def obj_by_id(self, id):
        return Obj(self.__state["objs"][id])

    def objs_by_team(self, team):
        return [self.obj_by_id(id) for id in self.ids_by_team(team)]

    def ids_by_team(self, team):
        if not isinstance(team, Team):
            raise TypeError("team must be a Team")
        return self.__state["teams"][team.value]

    def obj_by_loc(self, coord):
        id = self.id_by_loc(coord)
        return id and self.obj_by_id(id)

    def id_by_loc(self, coord):
        xs = self.__state["grid"][coord.x]
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
            raise TypeError("type must be an ActionType")
        if not isinstance(direction, Direction):
            raise TypeError("direction must be a Direction")
        self.type = type
        self.direction = direction

    def __repr__(self):
        return f"<Action: {self.type} {self.direction}"


def move(direction):
    return Action(ActionType.Move, direction)


def attack(direction):
    return Action(ActionType.Attack, direction)


import io


class _RobotRumbleLoggingIO(io.TextIOBase):
    def __init__(self, log):
        self._log = log

    def write(self, s):
        if self._log:
            self._log(s)
        else:
            raise RuntimeError(
                "unable to print; the current runner does not support logging"
            )
        return len(s)

    def flush(self):
        pass


del io


def _main(state, log=None):
    import sys

    sys.stdout = sys.stderr = _RobotRumbleLoggingIO(log)

    state = State(state)
    robot = globals().get("robot")
    if not isinstance(robot, type(_main)):
        raise TypeError("you must define a 'robot' function")
    if robot.__code__.co_argcount != 2:
        raise TypeError(
            "your robot function must accept 2 values: the current state "
            "and the details for the unit"
        )
    output = {}
    for id in state.ids_by_team(state.our_team):
        action = robot(state, state.obj_by_id(id))
        if not isinstance(action, Action):
            raise TypeError("your robot function must return an Action")
        output[id] = {"type": action.type.value, "direction": action.direction.value}
    return output
