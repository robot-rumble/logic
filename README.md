# robot rumble - logic backend

![][https://d3kx2398yo1gg8.cloudfront.net/images/demo.gif]

> https://robotrumble.org

Robot Rumble is a game where you code robots to battle other users' bots in an
arena. This is the logic backend, which is primarily written in Rust (using
`wasm-bindgen` in the browser and tokio + wasmer otherwise).

A detailed writeup of our architecture/infrastructure can be found
[here](https://rr-docs.readthedocs.io/en/latest/technical-details.html).

### Directory structure

- `logic/`: This contains the pure game logic of robot rumble. It mainly
  operates on the `RobotRunner` trait, which has a single method that takes in
  the state of the board and outputs a list of actions for each unit under its
  control.
- `lang-runners/`: wasm modules that implement our runner "ABI"/"protocol", for
  running user code in a sandboxed WebAssembly environment. Each runner
  receives JSON-serialized `ProgramInput` structs in stdin, and should print
  JSON-serialized `ProgramResult`s to stdout (types defined in `logic/`).
  - `lang-runners/javascript`: implements the
    [robot rumble API/environment](https://rr-docs.readthedocs.io/en/latest/index.html)
    for JavaScript, running JS code in the
    [quickjs](https://bellard.org/quickjs/) interpreter.
  - `lang-runners/python`: the same as above, but running Python in
    [RustPython](https://rustpython.github.io).
  - `lang-runners/lang-common.*`: "shared" functionality used for implementing
    `lang-runner`s.
- `env-runners/`: libraries or binaries that wrap the `logic` crate in order to
  run in environments like AWS lambda or the browser. Notably absent is the
  `rumblebot` CLI, which lives in its own repo at
  [robot-rumble/cli](https://github.com/robot-rumble/cli).
  - `env-runners/browser/`: a `wasm-bindgen` wasm module that runs robots as
    web workers, in conjunction with the portions of the
    [garage](https://github.com/robot-rumble/battle-viewer/blob/master/src/garage/match.worker.js)
    that are written in JS.
  - `env-runners/lambda`: an AWS lambda function that runs a battle between two
    robots and outputs the results to SQS for our backend server to process.
  - `env-runners/lambda-cache`: a tool to precompile the lang-runners to native
    code for running on lambda.
  - `env-runners/native`: a crate providing a `RobotRunner` that runs in tokio,
    with robots running as (blocking) tokio tasks. This is used by the cli and
    the `lambda` runner.
