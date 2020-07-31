#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")"

DEPLOY=
PROD=
for arg in "$@"; do
    case "$arg" in
        --deploy) DEPLOY=1 ;;
        --prod) PROD=1
    esac
done

tmpdir=$(mktemp -d)
trap 'rm -rf "$tmpdir"' EXIT SIGINT


TARGET_DIR="$PWD"/target/x86_64-unknown-linux-musl/release

if [[ $PROD ]]; then
    build_command=cargo
else
    build_command=cross
fi
unset OPENSSL_NO_VENDOR

[[ -x target/release/build-lambda-cache ]] || cargo build -p build-lambda-cache --release
target/release/build-lambda-cache "$tmpdir/wasmer-cache" wasm-dist/lang-runners/*
$build_command build -p lambda-runner --target=x86_64-unknown-linux-musl --all-features --release

cd "$tmpdir"

cp "$TARGET_DIR"/lambda-runner bootstrap
zip lambda.zip bootstrap
zip -r wasmer-cache.zip wasmer-cache/
rm bootstrap

if [[ $DEPLOY ]]; then
    aws s3 cp lambda.zip "s3://$S3_BUCKET"
    aws lambda update-function-code --function-name "$FUNCTION_NAME" --s3-bucket="$S3_BUCKET" --s3-key lambda.zip

    LAYER_NAME=robot-rumble-cached-wasm-runners
    aws s3 cp wasmer-cache.zip "s3://$S3_BUCKET"
    LAYER_ARN=$(aws lambda publish-layer-version --layer-name "$LAYER_NAME" --content=S3Bucket="$S3_BUCKET",S3Key=wasmer-cache.zip | jq -r .LayerVersionArn)
    aws lambda update-function-configuration --function-name "$FUNCTION_NAME" --layers "$LAYER_ARN"
fi
