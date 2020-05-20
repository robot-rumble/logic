#!/usr/bin/env bash

cd "$(dirname "$0")"

OUTDIR="$PWD"/webapp-dist
mkdir -p "$OUTDIR"

OPTIMIZE=
RUNNERS_ONLY=
for arg in "$@"; do
    case "$arg" in
        --optimize) OPTIMIZE=1; ;;
        --runners-only|-r) RUNNERS_ONLY=1
    esac
done

copy_lang() {
    fname=$(basename "$1")

    inf=$(realpath "$1")

    # # TODO: remove once we do this in the browser
    # wasm_transformer_cli "$inf"

    if [[ $OPTIMIZE ]]; then
        # Do this in order to work around some weird parsing bug in wasm-opt
        wasm-dis "$1" -o "$basename.wat"
        wasm-opt "$basename.wat" -Os -o "$1"
        rm "$basename.wat"
    fi

    mkdir -p "$OUTDIR/runners"
    cp "$1" "$OUTDIR/runners/$fname"
}

BOLD=$(printf '\033[1m')
NC=$(printf '\033[0m')

prepend() {
    while read line; do 
        echo "$BOLD$1$NC" "$line"
    done
}

pids=()

if [[ ! $RUNNERS_ONLY ]]; then
    {
        wasm-pack build runners/webapp
        cp -r runners/webapp/pkg "$OUTDIR/logic"
    } 2>&1 | prepend logic: &
    pids+=($!)
fi

{
    make -C langs/javascript
    copy_lang langs/javascript/jsrunner.wasm
} 2>&1 | prepend jsrunner: &
pids+=($!)


{
    cargo build --release --target wasm32-wasi --manifest-path=langs/python/Cargo.toml
    copy_lang ../target/wasm32-wasi/release/pyrunner.wasm
} 2>&1 | prepend pyrunner: &
pids+=($!)

for pid in "${pids[@]}"; do
    wait "$pid"
done
