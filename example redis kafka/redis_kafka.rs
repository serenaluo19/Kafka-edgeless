use edgeless_dataplane::core::Message;
use rdkafka::producer::{BaseProducer, BaseRecord};
use rdkafka::config::ClientConfig;
use anyhow::Result;

#[derive(Clone)]
pub struct KafkaResourceProvider {
    inner: std::sync::Arc<tokio::sync::Mutex<KafkaResourceProviderInner>>,
}

pub struct KafkaResourceProviderInner {
    resource_provider_id: edgeless_api::function_instance::InstanceId,
    dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
    instances: std::collections::HashMap<edgeless_api::function_instance::InstanceId, KafkaResource>,
}

pub struct KafkaResource {
    join_handle: tokio::task::JoinHandle<()>,
}

impl Drop for KafkaResource {
    fn drop(&mut self) {
        self.join_handle.abort();
    }
}

impl KafkaResource {
    async fn new(
        dataplane_handle: edgeless_dataplane::handle::DataplaneHandle,
        kafka_brokers: &str,
        kafka_topic: &str,
    ) -> Result<Self> {
        let mut dataplane_handle = dataplane_handle;
        let kafka_brokers = kafka_brokers.to_string();
        let kafka_topic = kafka_topic.to_string();

        let producer: BaseProducer = ClientConfig::new()
            .set("bootstrap.servers", &kafka_brokers)
            .create()?;

        log::info!("KafkaResource created, brokers: {}", kafka_brokers);

        let handle = tokio::spawn(async move {
            loop {
                let edgeless_dataplane::core::DataplaneEvent {
                    source_id,
                    channel_id,
                    message,
                } = dataplane_handle.receive_next().await;

                let mut need_reply = false;
                let message_data = match message {
                    Message::Call(data) => {
                        need_reply = true;
                        data
                    }
                    Message::Cast(data) => data,
                    _ => {
                        continue;
                    }
                };

                if let Err(e) = producer.send(
                    BaseRecord::to(&kafka_topic)
                        .payload(&message_data)
                        .key("some_key"),
                ) {
                    log::error!("Failed to send message to topic '{}': {}", kafka_topic, e);
                }

                if need_reply {
                    dataplane_handle
                        .reply(source_id, channel_id, edgeless_dataplane::core::CallRet::Reply("".to_string()))
                        .await;
                }
            }
        });

        Ok(Self { join_handle: handle })
    }
}

impl KafkaResourceProvider {
    pub async fn new(
        dataplane_provider: edgeless_dataplane::handle::DataplaneProvider,
        resource_provider_id: edgeless_api::function_instance::InstanceId,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(tokio::sync::Mutex::new(KafkaResourceProviderInner {
                resource_provider_id,
                dataplane_provider,
                instances: std::collections::HashMap::<edgeless_api::function_instance::InstanceId, KafkaResource>::new(),
            })),
        }
    }
}

#[async_trait::async_trait]
impl edgeless_api::resource_configuration::ResourceConfigurationAPI<edgeless_api::function_instance::InstanceId> for KafkaResourceProvider {
    async fn start(
        &mut self,
        instance_specification: edgeless_api::resource_configuration::ResourceInstanceSpecification,
    ) -> Result<edgeless_api::common::StartComponentResponse<edgeless_api::function_instance::InstanceId>> {
        if let (Some(brokers), Some(topic)) = (
            instance_specification.configuration.get("brokers"),
            instance_specification.configuration.get("topic"),
        ) {
            let mut lck = self.inner.lock().await;
            let new_id = edgeless_api::function_instance::InstanceId::new(lck.resource_provider_id.node_id);
            let dataplane_handle = lck.dataplane_provider.get_handle_for(new_id.clone()).await;

            match KafkaResource::new(dataplane_handle, brokers, topic).await {
                Ok(resource) => {
                    lck.instances.insert(new_id.clone(), resource);
                    return Ok(edgeless_api::common::StartComponentResponse::InstanceId(new_id));
                }
                Err(err) => {
                    return Ok(edgeless_api::common::StartComponentResponse::ResponseError(
                        edgeless_api::common::ResponseError {
                            summary: "Invalid resource configuration".to_string(),
                            detail: Some(err.to_string()),
                        },
                    ));
                }
            }
        }

        Ok(edgeless_api::common::StartComponentResponse::ResponseError(
            edgeless_api::common::ResponseError {
                summary: "Invalid resource configuration".to_string(),
                detail: Some("One of the fields 'brokers' or 'topic' is missing".to_string()),
            },
        ))
    }

    async fn stop(&mut self, resource_id: edgeless_api::function_instance::InstanceId) -> Result<()> {
        self.inner.lock().await.instances.remove(&resource_id);
        Ok(())
    }

    async fn patch(&mut self, _update: edgeless_api::common::PatchRequest) -> Result<()> {
        Ok(())
    }
}