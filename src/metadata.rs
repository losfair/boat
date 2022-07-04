use std::collections::HashMap;

use serde::Serialize;

use crate::config::AppConfig;

#[derive(Serialize)]
pub struct AppMetadata {
  pub env: HashMap<String, String>,
  pub secrets: HashMap<String, String>,
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
    }
  }
}
