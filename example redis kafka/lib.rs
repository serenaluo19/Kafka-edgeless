use edgeless_function::*;
use rdkafka::producer::{BaseProducer, BaseRecord};
use rdkafka::config::ClientConfig;
use anyhow::Result;

struct Counter;

impl EdgeFunction for Counter {

    fn handle_init(init_message: Option<&[u8]>, _serialized_state: Option<&[u8]>) {
        edgeless_function::init_logger();
        if let Some(init_message) = init_message {
            let init_msg_str = core::str::from_utf8(init_message).unwrap();
            let initial_value = match init_msg_str.parse::<i32>() {
                Ok(value) => value,
                Err(_) => 0,
            };
            if let Err(err) = start_kafka_producer(initial_value) {
                log::error!("Failed to start Kafka producer: {}", err);
            }
        }
    }

    fn handle_stop() {
        // noop
    }
}

fn start_kafka_producer(initial_value: i32) -> Result<()> {
    let kafka_brokers = "localhost:9092";  
    let kafka_topic = "counter-topic";   //si può modififcare con il topic desiderato

    let producer: BaseProducer = ClientConfig::new()
        .set("bootstrap.servers", kafka_brokers)
        .create()?;

    tokio::spawn(async move {
        let mut counter = initial_value;
        loop {
            let cur_count = format!("{}", counter);
            if let Err(e) = producer.send(
                BaseRecord::to(kafka_topic)
                    .payload(&cur_count)
                    .key("counter_key"),
            ) {
                log::error!("Failed to send message to topic '{}': {}", kafka_topic, e);
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            counter += 1;
        }
    });

    Ok(())
}

edgeless_function::export!(Counter);
