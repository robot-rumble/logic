"use strict";

import * as std from "std";

(function (g) {
  const source = std.loadFile(scriptArgs[1]);
  try {
    std.evalScript(source, { backtrace_barrier: true });
    if (typeof g.robot !== "function") {
      throw new TypeError("you must define a robot() function");
    }
  } catch (err) {
    print(
      "__rr_init:",
      JSON.stringify({
        Err: { InitError: convertErr(err) },
      })
    );
    std.exit(1);
  }
  print("__rr_init:", JSON.stringify({ Ok: null }));
  std.out.flush()

  let line;
  while ((line = std.in.getline())) {
    const input = JSON.parse(line);
    const output = runTurn(input);
    print("__rr_output:", JSON.stringify(output));
    std.out.flush();
  }

  function runTurn(state) {
    const outputs = {};
    for (const id of state.teams[state.team]) {
      const unit = state.objs[id];
      const debug_table = {};
      const debug = (k, v) => {
        debug_table[k] = v;
      };
      let action;
      try {
        action = { Ok: robot(state, unit, debug) };
      } catch (err) {
        action = { Err: formatErr(err) };
      }
      outputs[id] = { action, debug_table };
    }
    return { robot_outputs: { Ok: outputs }, logs: [] };
  }

  function convertErr(err) {
    return {
      start: [0, 0],
      end: [0, 0],
      message: err.toString(),
    };
  }
})(globalThis);
