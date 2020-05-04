#!/usr/bin/env bash

cd "$(dirname "$0")"

OPTIMIZE=
for arg in "$@"; do
    case "$arg" in
        --optimize) OPTIMIZE=1; ;;
    esac
done

optimize() {
    fname=$(basename "$1")

    # TODO: remove once we do this in the browser
    wasm_transformer_cli "$1"

    if [[ $OPTIMIZE ]]; then
        # Do this in order to work around some weird parsing bug in wasm-opt
        wasm-dis out.wasm -o out.wat
        wasm-opt out.wat -Os -o out.wasm
        rm out.wat
    fi

    cp out.wasm "../backend/src/runners/$fname"

    rm out.wasm
}

make -C langs/javascript

optimize langs/javascript/jsrunner.wasm

cargo build --release --target wasm32-wasi --manifest-path=langs/python/Cargo.toml

optimize target/wasm32-wasi/release/pyrunner.wasm
