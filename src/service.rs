use graphql_client::QueryBody;
use reqwest::{header::HeaderValue, Body, Method, Request, Url};
use serde::{Deserialize, Serialize};

use crate::authenticator::Credentials;

pub struct Service {
  client: reqwest::Client,
  creds: Option<Credentials>,
  endpoint: Url,
}

impl Service {
  pub fn new(endpoint: &str, credentials_file: &Option<String>) -> anyhow::Result<Self> {
    let creds = match Credentials::init(credentials_file) {
      Ok(creds) => Some(creds),
      Err(e) => {
        log::warn!("failed to load credentials: {}", e);
        None
      }
    };
    let endpoint =
      Url::parse(endpoint).map_err(|e| anyhow::Error::from(e).context("invalid endpoint url"))?;
    Ok(Service {
      client: reqwest::Client::new(),
      creds,
      endpoint,
    })
  }

  pub async fn call<V: Serialize, D: for<'de> Deserialize<'de>>(
    &self,
    query: QueryBody<V>,
  ) -> anyhow::Result<graphql_client::Response<D>> {
    let mut req = Request::new(Method::POST, self.endpoint.clone());
    {
      let headers = req.headers_mut();
      headers.insert("content-type", HeaderValue::from_static("application/json"));
      headers.insert("accept", HeaderValue::from_static("application/json"));
    }
    *req.body_mut() = Some(Body::from(serde_json::to_vec(&query)?));

    if let Some(creds) = &self.creds {
      creds.annotate_request(&mut req);
    }

    let res = self
      .client
      .execute(req)
      .await
      .map_err(|e| anyhow::Error::from(e).context("api call failed"))?;
    let status = res.status();
    if !status.is_success() {
      anyhow::bail!("api call returned error status: {}", status);
    }
    let body: graphql_client::Response<D> = res
      .json()
      .await
      .map_err(|e| anyhow::Error::from(e).context("api call failed"))?;
    Ok(body)
  }
}
