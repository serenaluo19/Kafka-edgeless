use rdkafka::{ClientConfig, Message};
use rdkafka::consumer::{CommitMode, consumer, StreamConsumer};

pub async fn start() {
    let consumer: StreamConsumer = create();
    consume(consumer).await
}

fn create()-> StreamConsumer {
    let mut config = ClientConfig::new()
    .set("bootstrap.servers", "localhost:9092")
    .set("auto.offset.reset", "earliest")
    .set("group.id", "test-group")
    .set("socket.timeout.ms", "4000");

    let consumer : StreamConsumer =
    config.create()
        .experct("Fail to create consumer");

    consumer
}

async fn consume(consumer: StreamConsumer) {
    consumer.subscribe(
        topics: &["test-topic"]
    ).experct("Can't Subscribe");

    loop {
        match consumer.recv().await {
            Err(e)=> println!("{:?}",e),
            Ok(message)=> {
                match message.payload_view::<str>() {
                    None => println!("Message consumed: {}", msg),
                    Some(Ok(msg))=> println!("Message Consumed : {}", msg),
                    Some(Err(e))=> println!("Error Parsing: {}", e)
                }
                consumer.commit_message(&message, CommitMode::Async).unwrap();
            }
        }
    }
}
