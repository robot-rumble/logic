#!/usr/bin/env bash

set -e

wasm-pack build

pushd ../../../backend/src
yarn build-worker
