use thiserror::Error;

use shared::{
    pubsub::{PublisherError, SubscriberError},
    sqlxmq::JobExecutionError,
};

#[allow(clippy::large_enum_variant)]
#[derive(Error, Debug)]
pub enum HedgingError {
    #[error("HedgingError - Subscriber: {0}")]
    Subscriber(#[from] SubscriberError),
    #[error("HedgingError - Publisher: {0}")]
    Publisher(#[from] PublisherError),
    #[error("HedgingError - SerdeJson: {0}")]
    SerdeJson(#[from] serde_json::Error),
    #[error("HedgingError - Sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("HedgingError - Migrate: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("HedgingError - OkexClient: {0}")]
    OkexClient(#[from] okex_client::OkexClientError),
}

impl JobExecutionError for HedgingError {}
