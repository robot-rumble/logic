#!/usr/bin/env bash

cd "$(dirname "$0")"

optimize() {
    fname=$(basename "$1")

    # TODO: remove once we do this in the browser
    wasm_transformer_cli "$1"

    wasm-opt out.wasm -Os -o "../backend/src/runners/$fname"

    rm out.wasm
}

make -C langs/javascript

optimize langs/javascript/jsrunner.wasm

cargo build --release --target wasm32-wasi --manifest-path=langs/python/Cargo.toml

optimize target/wasm32-wasi/release/pyrunner.wasm
