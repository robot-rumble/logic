#!/usr/bin/env bash

cd ../../target/debug/
aws --endpoint-url=http://localhost:4566 s3 cp lambda.zip s3://robot-runner-lambda
