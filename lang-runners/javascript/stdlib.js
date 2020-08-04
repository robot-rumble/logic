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
        return new Coords(0, 1)
      case Direction.South:
        return new Coords(0, -1)
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
    checkInstance(other, Coords, 'Coords.add')
    return new Coords(this.x + other.x, this.y + other.y)
  }

  sub(other) {
    checkInstance(other, Coords, 'Coords.sub')
    return new Coords(this.x - other.x, this.y - other.y)
  }

  mul(n) {
    checkType(n, 'number', 'Coords.mul')
    return new Coords(this.x * n, this.y * n)
  }
}

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

class ObjType extends Enum {}
ObjType.Unit = new ObjType()
ObjType.Terrain = new ObjType()
ObjType.closeEnum()

class Obj {
  constructor(obj) {
    checkType(obj, 'object', 'Obj constructor')
    this.__data = obj
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
    return this.__data.grid[coords.x]?.[coords.y]
  }

  objByCoords(coords) {
    checkInstance(coords, Coords, 'State.objByCoords')
    const id = this.idByCoords(coords)
    if (id) return this.objById(id)
  }
}

class ActionType extends Enum {}
ActionType.Attack = new ActionType()
ActionType.Move = new ActionType()
ActionType.closeEnum()

class Action {
  constructor(type, direction) {
    checkInstance(type, ActionType, 'Action constructor')
    checkInstance(direction, Direction, 'Action constructor')
    this.type = type
    this.direction = direction
  }

  toString() {
    return `<Action: ${this.type} ${this.direction}>`
  }

  static move(direction) {
    checkInstance(direction, Direction, 'Action.move')
    return new Action(ActionType.Move, direction)
  }

  static attack(direction) {
    checkInstance(direction, Direction, 'Action.attack')
    return new Action(ActionType.Attack, direction)
  }
}

function __format_err(err, is_init_err = false) {
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
  return { Err: is_init_err ? { InitError: e } : e }
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
    globalThis.initTurn(state)
  }

  const logs = []
  globalThis.console = {
    log(...args) {
      logs.push(args.join(' ') + '\n')
    },
  }
  const robot_actions = {}
  const debug_tables = {}
  const debug_inspections = []
  for (const id of state.idsByTeam(state.ourTeam)) {
    const debug_table = {}

    class Debug {
      log(key, val) {
        checkType(key, 'string', 'Debug.log "key"')
        debug_table[key] = String(val)
      }

      inspect(unit) {
        checkInstance(unit, Obj, 'Debug.inspect')
        debug_inspections.push(unit.id)
      }
    }

    globalThis.debug = new Debug()

    let result
    try {
      const output = globalThis.robot(state, state.objById(id))
      if (output instanceof Action) result = { Ok: output }
      else if (output === null) result = null
      else throw new TypeError('Robot must return an Action or null')
    } catch (e) {
      result = __format_err(e)
    }
    if (result !== null) robot_actions[id] = result
    debug_tables[id] = debug_table
  }
  return { Ok: { robot_actions, logs, debug_tables, debug_inspections } }
}
