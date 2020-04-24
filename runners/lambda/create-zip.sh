#!/usr/bin/env bash

cd ../../target/debug/
cp lambda bootstrap
zip lambda.zip bootstrap
rm bootstrap
