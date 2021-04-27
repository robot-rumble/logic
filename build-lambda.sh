#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")"

source _scripto.sh

DEPLOY=
PROD=
LAMBDA=
WASM_LAYER=
for arg in "$@"; do
    case "$arg" in
        --deploy) DEPLOY=1 ;;
        --prod) PROD=1 ;;
        --all|--lambda) LAMBDA=1 ;;&
        --all|--wasm-layer) WASM_LAYER=1 ;;&
    esac
done

ensure_some_target LAMBDA WASM_LAYER

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT SIGINT


if [[ $PROD ]]; then
    build_command=cargo
else
    build_command=cross
fi
unset OPENSSL_NO_VENDOR

pids=()

if [[ $WASM_LAYER ]]; then
    {
        cargo run -p lambda-cache --release "$tmpdir/wasmer-cache" wasm-dist/lang-runners/*
    } 2>&1 | prepend wasm-layer: &
    pids+=($!)
fi

if [[ $LAMBDA ]]; then
    {
        $build_command build -p lambda-runner --target=x86_64-unknown-linux-musl --all-features --release
        cp target/x86_64-unknown-linux-musl/release/lambda-runner "$tmpdir/bootstrap"
    } 2>&1 | prepend lambda-runner: &
    pids+=($!)
fi

wait_pids "${pids[@]}"

LAYER_NAME=wasmer-cache

if [[ $DEPLOY ]]; then
    cd "$tmpdir"

    if [[ $LAMBDA ]]; then
        zip lambda.zip bootstrap
        aws s3 cp lambda.zip "s3://$S3_BUCKET"
        aws lambda update-function-code --function-name "$FUNCTION_NAME" --s3-bucket="$S3_BUCKET" --s3-key lambda.zip
    fi

    if [[ $WASM_LAYER ]]; then
        zip -r wasmer-cache.zip wasmer-cache/
        aws s3 cp wasmer-cache.zip "s3://$S3_BUCKET"
        LAYER_ARN=$(aws lambda publish-layer-version --layer-name "$LAYER_NAME" --content=S3Bucket="$S3_BUCKET",S3Key=wasmer-cache.zip | jq -r .LayerVersionArn)
        aws lambda update-function-configuration --function-name "$FUNCTION_NAME" --layers "$LAYER_ARN"
    fi
fi
