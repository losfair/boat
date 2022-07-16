use std::collections::HashMap;

use serde::Serialize;

use crate::config::{AppConfig, MysqlMetadata, PubsubMetadata};

#[derive(Serialize)]
pub struct AppMetadata {
  pub env: HashMap<String, String>,
  pub secrets: HashMap<String, String>,
  pub mysql: HashMap<String, MysqlMetadata>,
  pub pubsub: HashMap<String, PubsubMetadata>,
}

impl AppMetadata {
  pub fn from_config(config: &AppConfig) -> Self {
    Self {
      env: config
        .env
        .iter()
        .map(|(k, v)| (k.get_ref().clone(), v.clone()))
        .collect(),
      secrets: config
        .secrets
        .iter()
        .map(|(k, v)| (k.get_ref().clone(), v.clone()))
        .collect(),
      mysql: config
        .mysql
        .iter()
        .map(|(k, v)| (k.get_ref().clone(), v.clone()))
        .collect(),
      pubsub: config
        .pubsub
        .iter()
        .map(|(k, v)| (k.get_ref().clone(), v.unwrap_as_metadata().clone()))
        .collect(),
    }
  }
}
