use std::collections::HashMap;

use anyhow::Result;
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

#[derive(Serialize)]
pub struct PackedAppMetadata {
  pub version: String,
  pub package: String,
  pub env: HashMap<String, String>,

  #[serde(default)]
  pub mysql: HashMap<String, MysqlMetadata>,

  #[serde(default)]
  pub pubsub: HashMap<String, PubsubMetadata>,
}

impl PackedAppMetadata {
  pub fn new(md: &AppMetadata, package_filename: &str) -> Result<Self> {
    let out = Self {
      version: "app".into(),
      package: package_filename.into(),
      env: md
        .env
        .iter()
        .chain(md.secrets.iter())
        .map(|x| (x.0.clone(), x.1.clone()))
        .collect(),
      mysql: md.mysql.clone(),
      pubsub: md.pubsub.clone(),
    };
    Ok(out)
  }
}
