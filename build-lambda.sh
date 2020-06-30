#!/usr/bin/env bash

set -e

cd "$(dirname "$0")"

DEPLOY=
PROD=
for arg in "$@"; do
    case "$arg" in
        --deploy) DEPLOY=1 ;;
        --prod) PROD=1
    esac
done

if [[ $PROD ]]; then
    build_command=cargo
else
    build_command=cross
fi
unset OPENSSL_NO_VENDOR
eval $build_command build -p lambda-runner --target=x86_64-unknown-linux-musl --all-features --release

pushd "target/x86_64-unknown-linux-musl/release"
cp lambda bootstrap
zip lambda.zip bootstrap
rm bootstrap

if [[ $DEPLOY ]]; then
    aws s3 cp lambda.zip s3://dev-battle-runner
    aws lambda update-function-code --function-name dev-battle-runner --s3-bucket=dev-battle-runner --s3-key lambda.zip
fi
