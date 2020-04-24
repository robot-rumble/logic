use lambda::lambda;
use rusoto_core::Region;
use rusoto_sqs::{SendMessageRequest, Sqs, SqsClient};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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

#[lambda]
#[tokio::main]
async fn main(data: Value) -> Result<(), Error> {
    let input_data = serde_json::from_value::<Input>(data).unwrap();

    let data = logic::MainOutput {
        winner: None,
        errors: HashMap::new(),
        turns: Vec::new(),
    };

    let client = SqsClient::new(Region::UsEast1);

    let out_queue_url = std::env::var("BATTLE_QUEUE_OUT_URL")
        .expect("\"BATTLE_QUEUE_OUT_URL\" environmental variable not found");

    let output = Output {
        r1_time: 0.,
        r1_id: input_data.r1_id,
        r2_time: 0.,
        r2_id: input_data.r2_id,
        data: serde_json::to_string(&data).unwrap(),
        winner: Winner::Draw,
        errored: false,
    };

    client.send_message(SendMessageRequest {
        queue_url: out_queue_url,
        message_body: serde_json::to_string(&output).unwrap(),
        delay_seconds: None,
        message_attributes: None,
        message_deduplication_id: None,
        message_group_id: None,
        message_system_attributes: None,
    });

    Ok(())
}
