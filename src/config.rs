use std::borrow::Cow;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use toml::Spanned;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppSpec {
  #[serde(default)]
  pub env: Vec<Spanned<EnvSpecOrPlain>>,
  #[serde(default)]
  pub secrets: Vec<Spanned<EnvSpecOrPlain>>,

  #[serde(default)]
  pub mysql: Vec<Spanned<String>>,

  pub build: Option<String>,

  #[serde(rename = "static")]
  pub _static: Option<String>,

  pub artifact: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MysqlMetadata {
  pub url: String,
  pub root_certificate: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum EnvSpecOrPlain {
  Spec(EnvSpec),
  Plain(String),
}

impl EnvSpecOrPlain {
  pub fn to_env_spec<'a>(&'a self) -> Cow<'a, EnvSpec> {
    match self {
      EnvSpecOrPlain::Spec(spec) => Cow::Borrowed(spec),
      EnvSpecOrPlain::Plain(name) => Cow::Owned(EnvSpec {
        key: name.clone(),
        regex: None,
        optional: false,
      }),
    }
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnvSpec {
  pub key: String,
  pub regex: Option<String>,
  #[serde(default)]
  pub optional: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
  pub id: String,
  #[serde(default)]
  pub env: IndexMap<Spanned<String>, String>,
  #[serde(default)]
  pub secrets: IndexMap<Spanned<String>, String>,
  #[serde(default)]
  pub mysql: IndexMap<Spanned<String>, MysqlMetadata>,
  #[serde(default)]
  pub detached_secrets: bool,
}
