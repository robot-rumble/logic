#!/usr/bin/env bash

# $1 specifies the cargo build folder
cd $1
cp lambda bootstrap
zip lambda.zip bootstrap
rm bootstrap
aws --region us-east-1 s3 cp lambda.zip s3://dev-battle-runner
aws --region us-east-1 lambda update-function-code --function-name dev-battle-runner --s3-bucket=dev-battle-runner --s3-key lambda.zip