#!/usr/bin/env bash
set -eo pipefail

cd "$(dirname "$0")"

pushd quickjs/src
make qjsc.wasm libquickjs.a AR=wasiar
QJSC=$PWD/qjsc.wasm
export CFLAGS="-I$PWD -L$PWD -lquickjs"
popd

wasmer run --dir . -- "$QJSC" -c -e -m -o jsrunner.c stdlib.js

wasicc $CFLAGS jsrunner.c -o jsrunner.wasm

