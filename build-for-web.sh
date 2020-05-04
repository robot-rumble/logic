#!/usr/bin/env bash

cd "$(dirname "$0")"

OUTDIR="$PWD"/webapp-dist
mkdir -p "$OUTDIR"

OPTIMIZE=
for arg in "$@"; do
    case "$arg" in
        --optimize) OPTIMIZE=1; ;;
    esac
done

copy_lang() {
    fname=$(basename "$1")

    # TODO: remove once we do this in the browser
    wasm_transformer_cli "$1"

    if [[ $OPTIMIZE ]]; then
        # Do this in order to work around some weird parsing bug in wasm-opt
        wasm-dis out.wasm -o out.wat
        wasm-opt out.wat -Os -o out.wasm
        rm out.wat
    fi

    mkdir -p "$OUTDIR/runners"
    cp out.wasm "$OUTDIR/runners/$fname"

    rm out.wasm
}

BOLD=$(printf '\033[1m')
NC=$(printf '\033[0m')

prepend() {
    while read line; do 
        echo "$BOLD$1$NC" "$line"
    done
}

pids=()

{
    wasm-pack build runners/webapp
    cp -r runners/webapp/pkg "$OUTDIR/logic"
} 2>&1 | prepend logic: &
pids+=($!)

{
    make -C langs/javascript
    copy_lang langs/javascript/jsrunner.wasm
} 2>&1 | prepend jsrunner: &
pids+=($!)


{
    cargo build --release --target wasm32-wasi --manifest-path=langs/python/Cargo.toml
    copy_lang target/wasm32-wasi/release/pyrunner.wasm
} 2>&1 | prepend pyrunner: &
pids+=($!)

for pid in "${pids[@]}"; do
    wait "$pid"
done
