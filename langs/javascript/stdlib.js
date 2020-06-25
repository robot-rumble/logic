// "use strict";

// https://2ality.com/2020/01/enum-pattern.html
// https://github.com/rauschma/enumify/blob/master/ts/src/index.ts
class Enum {
  static closeEnum () {
    const enumValues = []

    for (const [name, staticInstance] of Object.entries(this)) {
      staticInstance.value = name
      staticInstance.ordinal = enumValues.length
      enumValues.push(staticInstance)
    }

    // Important: only add more static properties *after* processing the enum entries
    this.enumValues = enumValues
  }

  static valueOf (str) {
    return this.enumValues.find(val => val.enumKey === str)
  }

  // INSTANCE

  toString () {
    return `${this.constructor.name}.${this.enumKey}`
  }
}

class Direction extends Enum {
  static East = new Direction()
  static West = new Direction()
  static South = new Direction()
  static North = new Direction()
  static _ = this.closeEnum()

  get opposite () {
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

  get toCoords () {
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

class Coords {
  constructor (x, y) {
    this.x = x
    this.y = y
  }

  toString () {
    return `(${this.x}, ${this.y})`
  }

  distance (other) {
    return Math.sqrt((other.x - this.x) ** 2 + (other.y - this.y) ** 2)
  }

  walkingDistance (other) {
    return Math.abs(other.x - this.x) + Math.abs(other.y - this.y)
  }

  towards (other) {
    const diff = this.sub(other)
    const angle = Math.atan2(diff.y, diff.x)
    if (Math.abs(angle) > Math.PI / 4) {
      if (diff.y > 0) return Direction.North
      else return Direction.South
    } else {
      if (diff.x > 0) return Direction.East
      else return Direction.West
    }
  }

  add (other) {
    return new Coords(this.x + other.x, this.y + other.y)
  }

  sub (other) {
    return new Coords(this.x - other.x, this.y - other.y)
  }

  mul (n) {
    return new Coords(this.x * n, this.y * n)
  }
}

class Team extends Enum {
  static Red = new Team()
  static Blue = new Team()
  static _ = this.closeEnum()

  get opposite () {
    if (this === Team.Red) {
      return Team.Blue
    } else return Team.Red
  }
}

class ObjType extends Enum {
  static Unit = new ObjType()
  static Terrain = new ObjType()
  static _ = this.closeEnum()
}

class Obj {
  constructor (obj) {
    this.__data = obj
  }

  get coords () {
    return new Coords(...this.__data['coords'])
  }

  get id () {
    return this.__data['id']
  }

  get objType () {
    return ObjType.valueOf(this.__data['obj_type'])
  }

  get team () {
    if (this.objType === ObjType.Unit) {
      return Team.valueOf(this.__data['team'])
    }
  }

  get health () {
    if (this.objType === ObjType.Unit) {
      return this.__data['health']
    }
  }
}

class State {
  constructor (state) {
    this.__data = state
  }

  get turnNum () {
    return this.__data['turnNum']
  }

  get ourTeam () {
    return Team.valueOf(this.__data['team'])
  }

  get otherTeam () {
    return this.ourTeam.opposite()
  }

  idsByTeam (team) {
    return this.__data['teams'][team.value]
  }

  objById (id) {
    return new Obj(this.__data['objs'][id])
  }

  objsByTeam (team) {
    return this.idsByTeam(team).map(this.objById)
  }

  idByCoords (coords) {
    return this.__data['grid'][coords.x][coords.y]
  }

  objByCoords (coords) {
    return this.objById(this.idByCoords(coords))
  }
}

class ActionType extends Enum {
  static Attack = new ActionType()
  static Move = new ActionType()
  static _ = this.closeEnum()
}

class Action {
  constructor (type, direction) {
    this.type = type
    this.direction = direction
  }

  toString () {
    return `<Action: ${this.type} ${this.direction}>`
  }

  static move (direction) {
    return new Action(ActionType.Move, direction)
  }

  static attack (direction) {
    return new Action(ActionType.Attack, direction)
  }
}

function __format_err (err, incl_err = false, init_err = false) {
  const e = {
    start: [0, 0],
    end: [0, 0],
    message: err.toString(),
  }
  return incl_err ? { Err: init_err ? { InitError: e } : e } : e
}


function __main (stateData) {
  function __validateFunction(f, argcount, mandatory) {
    if (typeof f !== "function") {
      if (mandatory) {
        throw new TypeError(`You must define a '${name}' function`)
      }
    } else {
      if (f.length !== argcount) {
        throw new TypeError(
          `Your ${name} function must accept ${argcount} arguments`
        )
      }
    }
  }

  const state = new State(stateData)

  try {
    __validateFunction(globalThis._robot, 3, true)
    __validateFunction(globalThis._initTurn, 1, true)
  } catch (e) {
    return {"robot_outputs": {"Err": {"InitError": __format_err(e)}}}
  }

  if (typeof globalThis._initTurn === 'function') {
    globalThis._initTurn(state)
  }

  const robotOutputs = {}
  for (const id of state.idsByTeam(state.ourTeam)) {
    const debug_table = {}

    const debug = (key, val) => {
      debug_table[key] = val
    }

    let result
    try {
      result = { Ok: globalThis._robot(stateData, state.objById(id), debug) }
    } catch (e) {
      result = { Err: __format_err(e) }
    }
    robotOutputs[id] = { action: result, debug_table }
  }
  return { robot_outputs: { Ok: robotOutputs }, logs: [] }
}
