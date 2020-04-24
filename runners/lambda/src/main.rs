#![allow(non_snake_case)]

use lambda::handler_fn;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/*
SAMPLE EVENT
{
    "Records": Array([Object({
        "attributes": Object({
            "ApproximateFirstReceiveTimestamp": String("1523232000001"),
            "ApproximateReceiveCount": String("1"),
            "SenderId": String("123456789012"),
            "SentTimestamp": String("1523232000000")
        }),
        "awsRegion": String("us-east-1"),
        "body": String("{\"r1_id\": 1, \"r1_code\": \"\", \"r2_id\": 2, \"r2_code\":\"\"}"),
        "eventSource": String("aws:sqs"),
        "eventSourceARN": String("arn:aws:sqs:us-east-1:123456789012:MyQueue"),
        "md5OfBody": String("7b270e59b47ff90a553787216d55d91d"),
        "messageAttributes": Object({}),
        "messageId": String("19dd0b57-b21e-4ac1-bd88-01bbb068cb78"),
        "receiptHandle": String("MessageReceiptHandle")
    })])
}
*/

#[derive(Deserialize)]
struct LambdaInput {
    Records: Vec<LambdaInputRecord>,
}

#[derive(Deserialize)]
struct LambdaInputRecord {
    body: String,
}

#[derive(Deserialize)]
struct Input {
    r1_id: usize,
    r1_code: String,
    r2_id: usize,
    r2_code: String,
}

#[derive(Serialize)]
enum Winner {
    R1,
    R2,
    Draw,
}

#[derive(Serialize)]
struct Output {
    r1_id: usize,
    r1_time: f64,
    r2_id: usize,
    r2_time: f64,
    data: String,
    winner: Winner,
    errored: bool,
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = handler_fn(handler);
    lambda::run(func).await
}

async fn handler(event: Value) -> Result<Value, Error> {
    let lambda_input = serde_json::from_value::<LambdaInput>(event).unwrap();
    let input_data = serde_json::from_str::<Input>(&lambda_input.Records[0].body).unwrap();

    let data = logic::MainOutput {
        winner: None,
        errors: HashMap::new(),
        turns: Vec::new(),
    };

    let output = Output {
        r1_time: 0.,
        r1_id: input_data.r1_id,
        r2_time: 0.,
        r2_id: input_data.r2_id,
        data: serde_json::to_string(&data).unwrap(),
        winner: Winner::Draw,
        errored: false,
    };

    Ok(serde_json::to_value(output).unwrap())
}
