#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")"

OUTDIR="$PWD"/wasm-dist
mkdir -p "$OUTDIR"

OPTIMIZE=
LANGS=
BROWSER=
for arg in "$@"; do
    case "$arg" in
        --optimize) OPTIMIZE=1; ;;
        --langs) LANGS=1 ;;
        --browser) BROWSER=1 ;;
        --all) LANGS=1; BROWSER=1
    esac
done

if [[ ! $LANGS && ! $BROWSER ]]; then
    echo "No build targets selected, exiting..."
    exit
fi


copy_lang() {
    fname=$(basename "$1")

    inf=$(realpath "$1")

    if [[ $OPTIMIZE ]]; then
        # Do this in order to work around some weird parsing bug in wasm-opt
        wasm-dis "$1" -o "$basename.wat"
        wasm-opt "$basename.wat" -Os -o "$1"
        rm "$basename.wat"
    fi

    mkdir -p "$OUTDIR/lang-runners"
    cp "$1" "$OUTDIR/lang-runners/$fname"
}

BOLD=$(printf '\033[1m')
NC=$(printf '\033[0m')

prepend() {
    while read line; do 
        echo "$BOLD$1$NC" "$line"
    done
}

pids=()

if [[ $BROWSER ]]; then
    {
        wasm-pack build env-runners/browser
        cp -r env-runners/browser/pkg "$OUTDIR/browser-runner"
    } 2>&1 | prepend browser-runner: &
    pids+=($!)
fi

if [[ $LANGS ]]; then
    {
        make -C lang-runners/javascript
        copy_lang lang-runners/javascript/jsrunner.wasm
    } 2>&1 | prepend jsrunner: &
    pids+=($!)


    {
        cargo build --release --target wasm32-wasi --manifest-path=lang-runners/python/Cargo.toml
        copy_lang target/wasm32-wasi/release/pyrunner.wasm
    } 2>&1 | prepend pyrunner: &
    pids+=($!)
fi

for pid in "${pids[@]}"; do
    wait "$pid"
done
