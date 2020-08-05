#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")"

source _scripto.sh

OPTIMIZE=
BROWSER=
PYTHON=
JAVASCRIPT=
for arg in "$@"; do
    case "$arg" in
        --optimize) OPTIMIZE=1 ;;
        --all|--browser) BROWSER=1 ;;&
        --all|--langs|--python) PYTHON=1 ;;&
        --all|--langs|--javascript) JAVASCRIPT=1 ;;&
    esac
done

ensure_some_target BROWSER PYTHON JAVASCRIPT

OUTDIR="$PWD"/wasm-dist
mkdir -p "$OUTDIR"

ansi_color JSYELLOWBG 48 2 240 219 79
ansi_color JSGRAY     38 2 50  51  48
ansi_color PYBLUEBG   48 2 48  105 152
ansi_color PYYELLOW   38 2 255 212 59

pids=()

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

if [[ $BROWSER ]]; then
    {
        wasm-pack build env-runners/browser
        cp -rT env-runners/browser/pkg "$OUTDIR/browser-runner"
    } 2>&1 | prepend browser-runner: &
    pids+=($!)
fi

if [[ $PYTHON ]]; then
    {
        cargo build -p pyrunner --target wasm32-wasi --release
        copy_lang target/wasm32-wasi/release/pyrunner.wasm
    } 2>&1 | prepend "$PYBLUEBG$PYYELLOW"pyrunner: &
    pids+=($!)

fi
if [[ $JAVASCRIPT ]]; then
    {
        make -C lang-runners/javascript
        copy_lang lang-runners/javascript/jsrunner.wasm
    } 2>&1 | prepend "$JSYELLOWBG$JSGRAY"jsrunner: &
    pids+=($!)
fi

wait_pids "${pids[@]}"
