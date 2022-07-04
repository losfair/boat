use std::{
  path::PathBuf,
  time::{SystemTime, UNIX_EPOCH},
};

use data_encoding::{BASE32_NOPAD, BASE64};
use ed25519_dalek::{ed25519::signature::Signature, Keypair, PublicKey, SecretKey, Signer};
use regex::Regex;
use reqwest::{header::HeaderValue, Request};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CredentialsJson {
  pub access_key: String,
  pub secret_key: String,
}

pub struct Credentials {
  ak: String,
  keypair: ed25519_dalek::Keypair,
}

impl Credentials {
  pub fn init(credentials_file: &Option<String>) -> anyhow::Result<Self> {
    let ak_regex = Regex::new(r#"^lha_([0-9a-z]{1,100})$"#).unwrap();
    let sk_regex = Regex::new(r#"^lhs_([0-9a-z]{1,100})$"#).unwrap();

    let (ak, sk) = if let (Ok(ak), Ok(sk)) = (
      std::env::var("BOAT_ACCESS_KEY"),
      std::env::var("BOAT_SECRET_KEY"),
    ) {
      (ak, sk)
    } else {
      let path = credentials_file
        .as_ref()
        .map(|x| PathBuf::from(x.as_str()))
        .unwrap_or_else(|| {
          dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/"))
            .join(".boat/credentials.json")
        });

      let raw_creds = std::fs::read(&path)
        .map_err(|e| anyhow::Error::from(e).context("cannot read credentials file"))?;

      let raw_creds: CredentialsJson = serde_json::from_slice(&raw_creds)
        .map_err(|e| anyhow::Error::from(e).context("cannot decode credentials file"))?;
      (raw_creds.access_key, raw_creds.secret_key)
    };

    if !ak_regex.is_match(&ak) {
      anyhow::bail!("invalid access key format");
    }

    if !sk_regex.is_match(&sk) {
      anyhow::bail!("invalid secret key format");
    }

    let ak_bin = BASE32_NOPAD
      .decode(ak.strip_prefix("lha_").unwrap().to_uppercase().as_bytes())
      .unwrap();

    let sk_bin = BASE32_NOPAD
      .decode(sk.strip_prefix("lhs_").unwrap().to_uppercase().as_bytes())
      .unwrap();

    if ak_bin.len() != 32 {
      anyhow::bail!("invalid access key length");
    }

    if sk_bin.len() != 32 {
      anyhow::bail!("invalid secret key length");
    }

    let sk = SecretKey::from_bytes(&sk_bin).unwrap();
    let computed_pubkey = PublicKey::from(&sk);
    if computed_pubkey.as_bytes() != &ak_bin[..] {
      anyhow::bail!("secret key does not match access key");
    }

    let keypair = Keypair {
      secret: sk,
      public: computed_pubkey,
    };

    Ok(Self { ak, keypair })
  }

  pub fn annotate_request(&self, req: &mut Request) {
    let current_time = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_secs();
    let sig = self.sign(current_time);
    let headers = req.headers_mut();
    headers.insert(
      "x-lighthouse-access-key",
      HeaderValue::from_str(&self.ak).unwrap(),
    );
    headers.insert(
      "x-lighthouse-request-time",
      HeaderValue::from_str(&format!("{}", current_time)).unwrap(),
    );
    headers.insert(
      "x-lighthouse-request-signature",
      HeaderValue::from_str(&sig).unwrap(),
    );
  }

  fn sign(&self, time_sec: u64) -> String {
    let payload = format!("request:{}", time_sec);
    let sig = self.keypair.sign(payload.as_bytes());
    BASE64.encode(sig.as_bytes())
  }
}
