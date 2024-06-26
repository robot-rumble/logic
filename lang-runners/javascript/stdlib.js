// "use strict";

function checkInstance(val, cls, funcName) {
  if (!(val instanceof cls)) {
    throw new TypeError(
      `${funcName} argument must be an instance of ${cls.name}`,
    )
  }
}

function checkType(val, type, funcName) {
  if (typeof val !== type) {
    throw new TypeError(`${funcName} argument must be of type ${type}`)
  }
}

// https://2ality.com/2020/01/enum-pattern.html
// https://github.com/rauschma/enumify/blob/master/ts/src/index.ts
class Enum {
  static closeEnum() {
    const enumValues = []

    for (const [name, staticInstance] of Object.entries(this)) {
      staticInstance.enumKey = name
      staticInstance.ordinal = enumValues.length
      enumValues.push(staticInstance)
    }

    // Important: only add more static properties *after* processing the enum entries
    this.enumValues = enumValues
  }

  static valueOf(str) {
    checkType(str, 'string', 'Enum.valueOf')
    return this.enumValues.find(val => val.enumKey === str)
  }

  // INSTANCE

  toString() {
    return `${this.constructor.name}.${this.enumKey}`
  }

  toJSON() {
    return this.enumKey
  }
}

class Direction extends Enum {
  get opposite() {
    switch (this) {
      case Direction.East:
        return Direction.West
      case Direction.West:
        return Direction.East
      case Direction.North:
        return Direction.South
      case Direction.South:
        return Direction.North
    }
  }

  get toCoords() {
    switch (this) {
      case Direction.East:
        return new Coords(1, 0)
      case Direction.West:
        return new Coords(-1, 0)
      case Direction.North:
        return new Coords(0, -1)
      case Direction.South:
        return new Coords(0, 1)
    }
  }

  get rotateCw() {
    switch (this) {
      case Direction.North:
        return Direction.East
      case Direction.East:
        return Direction.South
      case Direction.South:
        return Direction.West
      case Direction.West:
        return Direction.North
    }
  }

  get rotateCcw() {
    switch (this) {
      case Direction.North:
        return Direction.West
      case Direction.West:
        return Direction.South
      case Direction.South:
        return Direction.East
      case Direction.East:
        return Direction.North
    }
  }
}
Direction.East = new Direction()
Direction.West = new Direction()
Direction.South = new Direction()
Direction.North = new Direction()
Direction.closeEnum()

class Coords {
  constructor(x, y) {
    checkType(x, 'number', 'Coords constructor')
    checkType(y, 'number', 'Coords constructor')
    this.x = x
    this.y = y
  }

  isSpawn() {
    return SPAWN_COORDS_STRINGS.has(this.toString())
  }

  isHill() {
    return HILL_COORDS_STRINGS.has(this.toString())
  }

  toString() {
    return `(${this.x}, ${this.y})`
  }

  distanceTo(other) {
    checkInstance(other, Coords, 'Coords.distanceTo')
    return Math.sqrt((other.x - this.x) ** 2 + (other.y - this.y) ** 2)
  }

  walkingDistanceTo(other) {
    checkInstance(other, Coords, 'Coords.walkingDistanceTo')
    return Math.abs(other.x - this.x) + Math.abs(other.y - this.y)
  }

  directionTo(other) {
    checkInstance(other, Coords, 'Coords.directionTo')
    const diff = this.sub(other)
    const angle = Math.atan2(diff.y, diff.x)
    if (Math.abs(angle) <= Math.PI / 4) {
      return Direction.West
    } else if (Math.abs(angle + Math.PI / 2) <= Math.PI / 4) {
      return Direction.South
    } else if (Math.abs(angle - Math.PI / 2) <= Math.PI / 4) {
      return Direction.North
    } else {
      return Direction.East
    }
  }

  add(other) {
    if (other instanceof Coords) {
      return new Coords(this.x + other.x, this.y + other.y)
    } else if (other instanceof Direction) {
      return new Coords(this.x + other.toCoords.x, this.y + other.toCoords.y)
    } else {
      throw new TypeError('Coords.add argument must be an instance of Coords or Direction')
    }
  }

  sub(other) {
    if (other instanceof Coords) {
      return new Coords(this.x - other.x, this.y - other.y)
    } else if (other instanceof Direction) {
      return new Coords(this.x - other.toCoords.x, this.y - other.toCoords.y)
    } else {
      throw new TypeError('Coords.sub argument must be an instance of Coords or Direction')
    }
  }

  mul(n) {
    checkType(n, 'number', 'Coords.mul')
    return new Coords(this.x * n, this.y * n)
  }
}

const SPAWN_COORDS = new Set([new Coords(1, 5), new Coords(1, 6), new Coords(1, 7), new Coords(1, 8), new Coords(1, 9), new Coords(1, 10), new Coords(1, 11), new Coords(1, 12), new Coords(1, 13), new Coords(2, 4), new Coords(2, 14), new Coords(3, 3), new Coords(3, 15), new Coords(4, 2), new Coords(4, 16), new Coords(5, 1), new Coords(5, 17), new Coords(6, 1), new Coords(6, 17), new Coords(7, 1), new Coords(7, 17), new Coords(8, 1), new Coords(8, 17), new Coords(9, 1), new Coords(9, 17), new Coords(10, 1), new Coords(10, 17), new Coords(11, 1), new Coords(11, 17), new Coords(12, 1), new Coords(12, 17), new Coords(13, 1), new Coords(13, 17), new Coords(14, 2), new Coords(14, 16), new Coords(15, 3), new Coords(15, 15), new Coords(16, 4), new Coords(16, 14), new Coords(17, 5), new Coords(17, 6), new Coords(17, 7), new Coords(17, 8), new Coords(17, 9), new Coords(17, 10), new Coords(17, 11), new Coords(17, 12), new Coords(17, 13)])
const SPAWN_COORDS_STRINGS = new Set([...SPAWN_COORDS].map(coords => coords.toString()))

const HILL_COORDS = new Set([new Coords(9, 9), new Coords(8, 9), new Coords(8, 8), new Coords(9, 8), new Coords(10, 8), new Coords(10, 9), new Coords(10, 10), new Coords(9, 10), new Coords(8, 10)])
const HILL_COORDS_STRINGS = new Set([...HILL_COORDS].map(coords => coords.toString()))

class Team extends Enum {
  get opposite() {
    if (this === Team.Red) {
      return Team.Blue
    } else return Team.Red
  }
}
Team.Red = new Team()
Team.Blue = new Team()
Team.closeEnum()

class ObjType extends Enum { }
ObjType.Unit = new ObjType()
ObjType.Terrain = new ObjType()
ObjType.closeEnum()

class Obj {
  constructor(obj) {
    checkType(obj, 'object', 'Obj constructor')
    this.__data = obj
  }

  toString() {
    if (this.objType === ObjType.Unit)
      return `<${this.objType} id=${this.id} coords=${this.coords} ${this.team} health=${this.health}>`
    else
      return `<${this.objType} id=${this.id} coords=${this.coords}>`
  }

  get coords() {
    return new Coords(...this.__data.coords)
  }

  get id() {
    return this.__data.id
  }

  get objType() {
    return ObjType.valueOf(this.__data.obj_type)
  }

  get team() {
    if (this.objType === ObjType.Unit) {
      return Team.valueOf(this.__data.team)
    }
  }

  get health() {
    if (this.objType === ObjType.Unit) {
      return this.__data.health
    }
  }
}

class State {
  constructor(state) {
    checkType(state, 'object', 'State constructor')
    this.__data = state
  }

  get turn() {
    return this.__data.turn
  }

  get ourTeam() {
    return Team.valueOf(this.__data.team)
  }

  get otherTeam() {
    return this.ourTeam.opposite
  }

  objById(id) {
    checkType(id, 'string', 'State.objById')
    const obj = this.__data.objs[id]
    if (obj) return new Obj(obj)
  }

  idsByTeam(team) {
    checkInstance(team, Team, 'State.idsByTeam')
    return this.__data.teams[team.enumKey]
  }

  objsByTeam(team) {
    checkInstance(team, Team, 'State.objsByTeam')
    return this.idsByTeam(team).map(id => this.objById(id))
  }

  idByCoords(coords) {
    checkInstance(coords, Coords, 'State.idByCoords')
    return this.__data.grid[coords.y]?.[coords.x]
  }

  objByCoords(coords) {
    checkInstance(coords, Coords, 'State.objByCoords')
    const id = this.idByCoords(coords)
    if (id) return this.objById(id)
  }
}

class ActionType extends Enum { }
ActionType.Attack = new ActionType()
ActionType.Move = new ActionType()
ActionType.Heal = new ActionType()
ActionType.closeEnum()

class Action {
  constructor(type, direction) {
    checkInstance(type, ActionType, 'Action constructor')
    checkInstance(direction, Direction, 'Action constructor')
    this.type = type
    this.direction = direction
  }

  toString() {
    return `<${this.type} ${this.direction}>`
  }

  static move(direction) {
    checkInstance(direction, Direction, 'Action.move')
    return new Action(ActionType.Move, direction)
  }

  static attack(direction) {
    checkInstance(direction, Direction, 'Action.attack')
    return new Action(ActionType.Attack, direction)
  }

  static heal(direction) {
    checkInstance(direction, Direction, 'Action.heal')
    return new Action(ActionType.Heal, direction)
  }
}


const MAP_SIZE = 19


function __format_err(err, isInitError = false) {
  let lineno = null
  if (err) {
    if (err.lineNumber) {
      lineno = err.lineNumber
    } else if (Array.isArray(err.traceback)) {
      for (const entry of err.traceback) {
        if (
          entry &&
          entry.fileName === '<robot>' &&
          Number.isInteger(entry.lineNumber)
        ) {
          lineno = entry.lineNumber
        }
      }
    }
  }
  const e = {
    loc:
      lineno == null
        ? null
        : {
          start: [lineno, null],
          end: null,
        },
    summary: String(err),
    details: (err && err.stack) || null,
  }
  return { Err: isInitError ? { InitError: e } : e }
}

function __main(stateData) {
  function __validateFunction(name, f, argcount, mandatory) {
    if (typeof f !== 'function') {
      if (mandatory) {
        throw new TypeError(`You must define a '${name}' function`)
      }
    } else {
      if (f.length !== argcount) {
        const err = new TypeError(
          `Your ${name} function must accept ${argcount} arguments`,
        )
        err.lineNumber = f && f.lineNumber
        err.fileName = f && f.fileName
        throw err
      }
    }
  }

  const state = new State(stateData)

  try {
    __validateFunction('robot', globalThis.robot, 2, true)
    __validateFunction('initTurn', globalThis.initTurn, 1, false)
  } catch (e) {
    return __format_err(e, true)
  }

  if (typeof globalThis.initTurn === 'function') {
    try {
      globalThis.initTurn(state)
    } catch (e) {
      return __format_err(e, true)
    }
  }

  const logs = []
  globalThis.console = {
    log(...args) {
      logs.push(args.join(' ') + '\n')
    },
  }
  const robot_actions = {}
  const debug_inspect_tables = {}
  const debug_locate_queries = []
  for (const id of state.idsByTeam(state.ourTeam)) {
    const debug_inspect_table = {}

    class Debug {
      inspect(key, val) {
        checkType(key, 'string', 'Debug.inspect "key"')
        debug_inspect_table[key] = String(val)
      }

      locate(unit) {
        checkInstance(unit, Obj, 'Debug.locate')
        debug_locate_queries.push(unit.id)
      }
    }

    globalThis.debug = new Debug()

    let result
    try {
      const output = globalThis.robot(state, state.objById(id))
      if (output instanceof Action) result = { Ok: output }
      else if (output === null) result = { Ok: null }
      else throw new TypeError('Robot must return an Action or null')
    } catch (e) {
      result = __format_err(e)
    }
    robot_actions[id] = result
    if (Object.keys(debug_inspect_table).length) debug_inspect_tables[id] = debug_inspect_table
  }
  return { Ok: { robot_actions, logs, debug_inspect_tables, debug_locate_queries } }
}
