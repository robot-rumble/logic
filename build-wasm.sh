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
        --optimize) OPTIMIZE=1 ;;
        --langs) LANGS=1 ;;
        --browser) BROWSER=1 ;;
        --all)
            LANGS=1
            BROWSER=1
            ;;
    esac
done

if [[ ! $LANGS && ! $BROWSER ]]; then
    echo "No build targets selected, exiting..."
    exit
fi

copy_lang() {
    fname=$(basename "$1")

    if [[ $OPTIMIZE ]]; then
        # Do this in order to work around some weird parsing bug in wasm-opt
        wasm-dis "$1" -o "$fname.wat"
        wasm-opt "$fname.wat" -Os -o "$1"
        rm "$fname.wat"
    fi

    mkdir -p "$OUTDIR/lang-runners"
    cp "$1" "$OUTDIR/lang-runners/$fname"
}

BOLD=$(printf '\033[1m')
NC=$(printf '\033[0m')
JSYELLOWBG=$(printf '\033[48;2;240;219;79m')
JSGRAY=$(printf '\033[38;2;50;51;48m')
PYBLUEBG=$(printf '\033[48;2;48;105;152m')
PYYELLOW=$(printf '\033[38;2;255;212;59m')

prepend() {
    while IFS='' read -r line; do
        echo "$BOLD$1$NC" "$line"
    done
}

pids=()

if [[ $BROWSER ]]; then
    {
        wasm-pack build env-runners/browser
        rm -r "$OUTDIR/browser-runner"
        cp -r env-runners/browser/pkg "$OUTDIR/browser-runner"
    } 2>&1 | prepend browser-runner: &
    pids+=($!)
fi

if [[ $LANGS ]]; then
    {
        make -C lang-runners/javascript
        copy_lang lang-runners/javascript/jsrunner.wasm
    } 2>&1 | prepend "$JSYELLOWBG$JSGRAY"jsrunner: &
    pids+=($!)

    {
        cargo build --release --target wasm32-wasi --manifest-path=lang-runners/python/Cargo.toml
        copy_lang target/wasm32-wasi/release/pyrunner.wasm
    } 2>&1 | prepend "$PYBLUEBG$PYYELLOW"pyrunner: &
    pids+=($!)
fi

code=0

for pid in "${pids[@]}"; do
    wait "$pid" || code=1
done

exit $code
