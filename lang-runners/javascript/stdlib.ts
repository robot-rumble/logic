// "use strict";

/// <reference path="logictypes.d.ts" />
import type * as types from 'logictypes'

function checkInstance<T>(
  val: T,
  cls: new (...args: any) => T,
  funcName: string,
) {
  if (!(val instanceof cls)) {
    throw new TypeError(
      `${funcName} argument must be an instance of ${cls.name}`,
    )
  }
}

function checkType<T>(
  val: T,
  type: string,
  funcName: string,
): asserts val is T {
  if (typeof val !== type) {
    throw new TypeError(`${funcName} argument must be of type ${type}`)
  }
}

// https://2ality.com/2020/01/enum-pattern.html
// https://github.com/rauschma/enumify/blob/master/ts/src/index.ts
class Enum {
  static enumValues: Enum[]
  enumKey!: string
  ordinal!: number
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

  static valueOf<T extends typeof Enum>(
    this: T,
    str: string,
  ): InstanceType<T> | undefined {
    checkType(str, 'string', 'Enum.valueOf')
    return this.enumValues.find(val => val.enumKey === str) as any
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
  get opposite(): Direction {
    switch (this) {
      case Direction.East:
        return Direction.West
      case Direction.West:
        return Direction.East
      case Direction.North:
        return Direction.South
      case Direction.South:
        return Direction.North
      default:
        throw new Error('invalid coord')
    }
  }

  get toCoords(): Coords {
    switch (this) {
      case Direction.East:
        return new Coords(1, 0)
      case Direction.West:
        return new Coords(-1, 0)
      case Direction.North:
        return new Coords(0, -1)
      case Direction.South:
        return new Coords(0, 1)
      default:
        throw new Error('invalid coord')
    }
  }

  get rotateCw(): Direction {
    switch (this) {
      case Direction.North:
        return Direction.East
      case Direction.East:
        return Direction.South
      case Direction.South:
        return Direction.West
      case Direction.West:
        return Direction.North
      default:
        throw new Error('invalid coord')
    }
  }

  get rotateCcw(): Direction {
    switch (this) {
      case Direction.North:
        return Direction.West
      case Direction.West:
        return Direction.South
      case Direction.South:
        return Direction.East
      case Direction.East:
        return Direction.North
      default:
        throw new Error('invalid coord')
    }
  }
}
namespace Direction {
  export const East = new Direction()
  export const West = new Direction()
  export const South = new Direction()
  export const North = new Direction()
}
Direction.closeEnum()

class Coords {
  constructor(public x: number, public y: number) {
    checkType(x, 'number', 'Coords constructor')
    checkType(y, 'number', 'Coords constructor')
  }

  toString() {
    return `(${this.x}, ${this.y})`
  }

  distanceTo(other: Coords) {
    checkInstance(other, Coords, 'Coords.distanceTo')
    return Math.sqrt((other.x - this.x) ** 2 + (other.y - this.y) ** 2)
  }

  walkingDistanceTo(other: Coords) {
    checkInstance(other, Coords, 'Coords.walkingDistanceTo')
    return Math.abs(other.x - this.x) + Math.abs(other.y - this.y)
  }

  directionTo(other: Coords) {
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

  add(other: Coords | Direction) {
    if (other instanceof Coords) {
      return new Coords(this.x + other.x, this.y + other.y)
    } else if (other instanceof Direction) {
      return new Coords(this.x + other.toCoords.x, this.y + other.toCoords.y)
    } else {
      throw new TypeError(
        'Coords.add argument must be an instance of Coords or Direction',
      )
    }
  }

  sub(other: Coords | Direction) {
    if (other instanceof Coords) {
      return new Coords(this.x - other.x, this.y - other.y)
    } else if (other instanceof Direction) {
      return new Coords(this.x - other.toCoords.x, this.y - other.toCoords.y)
    } else {
      throw new TypeError(
        'Coords.sub argument must be an instance of Coords or Direction',
      )
    }
  }

  mul(n: number) {
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
namespace Team {
  export const Red = new Team()
  export const Blue = new Team()
}
Team.closeEnum()

class ObjType extends Enum {}
namespace ObjType {
  export const Unit = new ObjType()
  export const Terrain = new ObjType()
}
ObjType.closeEnum()

class Obj {
  constructor(private __data: types.Obj) {
    checkType(__data, 'object', 'Obj constructor')
  }

  toString() {
    if (this.objType === ObjType.Unit)
      return `<${this.objType} id=${this.id} coords=${this.coords} ${this.team} health=${this.health}>`
    else return `<${this.objType} id=${this.id} coords=${this.coords}>`
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
      return Team.valueOf(this.__data.team as string)
    }
  }

  get health() {
    if (this.objType === ObjType.Unit) {
      return this.__data.health
    }
  }
}

class State {
  constructor(private __data: types.ProgramInput) {
    checkType(__data, 'object', 'State constructor')
  }

  get turn() {
    return this.__data.turn
  }

  get ourTeam() {
    return Team.valueOf(this.__data.team)!
  }

  get otherTeam() {
    return this.ourTeam.opposite
  }

  objById(id: string) {
    checkType(id, 'string', 'State.objById')
    const obj = this.__data.objs[id]
    if (obj) return new Obj(obj)
  }

  idsByTeam(team: Team) {
    checkInstance(team, Team, 'State.idsByTeam')
    return this.__data.teams[team.enumKey]
  }

  objsByTeam(team: Team) {
    checkInstance(team, Team, 'State.objsByTeam')
    return this.idsByTeam(team).map(id => this.objById(id))
  }

  idByCoords(coords: Coords) {
    checkInstance(coords, Coords, 'State.idByCoords')
    return this.__data.grid[coords.y]?.[coords.x]
  }

  objByCoords(coords: Coords) {
    checkInstance(coords, Coords, 'State.objByCoords')
    const id = this.idByCoords(coords)
    if (id) return this.objById(id)
  }
}

class ActionType extends Enum {}
namespace ActionType {
  export const Attack = new ActionType()
  export const Move = new ActionType()
}
ActionType.closeEnum()

class Action {
  constructor(public type: ActionType, public direction: Direction) {
    checkInstance(type, ActionType, 'Action constructor')
    checkInstance(direction, Direction, 'Action constructor')
  }

  toString() {
    return `<${this.type} ${this.direction}>`
  }

  static move(direction: Direction) {
    checkInstance(direction, Direction, 'Action.move')
    return new Action(ActionType.Move, direction)
  }

  static attack(direction: Direction) {
    checkInstance(direction, Direction, 'Action.attack')
    return new Action(ActionType.Attack, direction)
  }
}

const MAP_SIZE = 19

function __format_err(err: any, init_err?: false): { Err: types.Error }
function __format_err(err: any, init_err: true): { Err: types.ProgramError }
function __format_err(
  err: any,
  init_err: boolean = false,
): { Err: types.Error | types.ProgramError } {
  let lineno: number | null = null
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
  const e: types.Error = {
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
  return { Err: init_err ? { InitError: e } : e }
}

interface HasLineFile {
  lineNumber?: number
  fileName?: string
}
declare global {
  interface Error extends HasLineFile {}
  interface Function extends HasLineFile {}
}

function __main(stateData: types.ProgramInput): types.ProgramResult {
  function __validateFunction(
    name: string,
    f: unknown,
    argcount: number,
    mandatory: false,
  ): Function | undefined
  function __validateFunction(
    name: string,
    f: unknown,
    argcount: number,
    mandatory: true,
  ): Function
  function __validateFunction(
    name: string,
    f: unknown,
    argcount: number,
    mandatory: boolean,
  ): Function | undefined {
    if (typeof f !== 'function') {
      if (mandatory) {
        throw new TypeError(`You must define a '${name}' function`)
      }
    } else {
      if (f.length !== argcount) {
        const err = new TypeError(
          `Your ${name} function must accept ${argcount} arguments`,
        ) as any
        err.lineNumber = f && f.lineNumber
        err.fileName = f && f.fileName
        throw err
      }
      return f
    }
  }

  const state = new State(stateData)

  let robot: Function
  let initTurn: Function | undefined
  try {
    robot = __validateFunction('robot', (globalThis as any).robot, 2, true)!
    initTurn = __validateFunction(
      'initTurn',
      (globalThis as any).initTurn,
      1,
      false,
    )
  } catch (e) {
    return __format_err(e, true)
  }

  if (initTurn) {
    try {
      initTurn(state)
    } catch (e) {
      return __format_err(e, true)
    }
  }

  const logs: string[] = []
  const log = (...args: any) => {
    logs.push(args.join(' ') + '\n')
  }
  const logPrefix = (s: string) => (...args: any[]) => log(s, args)
  // @ts-ignore
  globalThis.console = {
    log,
    debug: logPrefix('debug:'),
    info: logPrefix('info:'),
  }
  const robot_actions: types.ProgramOutput['robot_actions'] = {}
  const debug_tables: types.ProgramOutput['debug_tables'] = {}
  const debug_inspections: string[] = []
  for (const id of state.idsByTeam(state.ourTeam)) {
    const debug_table: { [k: string]: string } = {}

    class Debug {
      log(key: string, val: any) {
        checkType(key, 'string', 'Debug.log "key"')
        debug_table[key] = String(val)
      }

      inspect(unit: Obj) {
        checkInstance(unit, Obj, 'Debug.inspect')
        debug_inspections.push(unit.id)
      }
    }

    ;(globalThis as any).debug = new Debug()

    let result: types.ResultOf_ActionOr_Error
    try {
      const output = robot(state, state.objById(id))
      if (output instanceof Action) result = { Ok: output }
      else if (output === null) result = { Ok: null }
      else throw new TypeError('Robot must return an Action or null')
    } catch (e) {
      result = __format_err(e)
    }
    robot_actions[id] = result
    if (Object.keys(debug_table).length) debug_tables[id] = debug_table
  }
  return { Ok: { robot_actions, logs, debug_tables, debug_inspections } }
}
