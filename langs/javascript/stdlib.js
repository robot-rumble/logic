// "use strict";

// TODO: after initializing user code, ensure that `robot` is a function

function __main(state) {
  // oldconsole =
  const outputs = {}
  for (const id of state.teams[state.team]) {
    const unit = state.objs[id]
    const debug_table = {}
    const debug = (k, v) => {
      debug_table[k] = v
    }
    let action
    try {
      action = { Ok: globalThis.robot(state, unit, debug) }
    } catch (err) {
      action = { Err: __format_err(err) }
    }
    outputs[id] = { action, debug_table }
  }
  return { robot_outputs: { Ok: outputs }, logs: [] }
}

function __format_err(err, incl_err = false, init_err = false) {
  const e = {
    start: [0, 0],
    end: [0, 0],
    message: err.toString(),
  }
  return incl_err ? { Err: init_err ? { InitError: e } : e } : e
}

const East = 'East'
const West = 'West'
const South = 'South'
const North = 'North'
const directions = [East, West, South, North]

function move(direction) {
  if (!directions.includes(direction)) {
    throw new TypeError('Invalid direction!')
  }
  return { type: 'Move', direction }
}
function attack(direction) {
  if (!directions.includes(direction)) {
    throw new TypeError('Invalid direction!')
  }
  return { type: 'Attack', direction }
}
