mod producer;
mod consumer;

fn main() {
    let producer= producer::create();
    produce(producer, String::from("Hello World")).await;

    consumer::start().await;
}