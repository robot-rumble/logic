use lambda::lambda;
use rusoto_core::Region;
use rusoto_sqs::{SendMessageRequest, Sqs, SqsClient};
use serde_json::Value;

struct LambdaInput {}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

#[lambda]
#[tokio::main]
async fn main(event: Value) -> Result<Value, Error> {
    let client = SqsClient::new(Region::UsEast1);

    let queue_url = std::env::var("SQS_URL").expect("\"SQS_URL\" environmental variable not found");

    client.send_message(SendMessageRequest {
        queue_url,
        message_body: "".into(),
        delay_seconds: None,
        message_attributes: None,
        message_deduplication_id: None,
        message_group_id: None,
        message_system_attributes: None,
    })
}
