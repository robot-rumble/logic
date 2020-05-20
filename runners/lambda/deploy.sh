#!/usr/bin/env bash

set -e

cd "$(dirname "$0")"

if [[ $# -gt 0 ]]; then
    target_dir=$1
else
    cargo build --release
    target_dir=$(cargo metadata --format-version=1 | jq -r .target_directory)/release
fi

pushd "$target_dir"
cp lambda bootstrap
zip lambda.zip bootstrap
rm bootstrap
aws --region us-east-1 s3 cp lambda.zip s3://dev-battle-runner
aws --region us-east-1 lambda update-function-code --function-name dev-battle-runner --s3-bucket=dev-battle-runner --s3-key lambda.zip
