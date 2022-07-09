use graphql_client::{GraphQLQuery, QueryBody};
use reqwest::{header::HeaderValue, Body, Method, Request, Url};
use serde::{Deserialize, Serialize};
use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

use crate::{
  authenticator::Credentials,
  metadata::AppMetadata,
  schema::{self, RunDeploymentCreation, RunDeploymentPreparation},
};

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

  pub async fn deploy(
    &self,
    app_id: &str,
    metadata: &AppMetadata,
    package: &[u8],
  ) -> anyhow::Result<()> {
    let q = RunDeploymentPreparation::build_query(schema::run_deployment_preparation::Variables {
      app_id: app_id.to_string(),
    });
    let rsp = self
      .call::<_, schema::run_deployment_preparation::ResponseData>(q)
      .await?
      .check_service_error()?;
    let prep = rsp
      .data
      .as_ref()
      .map(|x| &x.prepare_deployment)
      .ok_or_else(|| anyhow::anyhow!("missing data in prep"))?;
    log::info!("uploading to s3: {}", prep.url);
    let s3_rsp = self
      .client
      .put(prep.url.as_str())
      .body(package.to_vec())
      .send()
      .await?;
    let s3_status = s3_rsp.status();
    if !s3_status.is_success() {
      anyhow::bail!("s3 upload failed: {}", s3_status);
    }
    let metadata = serde_json::to_string(metadata)?;
    log::info!("committing deployment");
    let q = RunDeploymentCreation::build_query(schema::run_deployment_creation::Variables {
      app_id: app_id.to_string(),
      metadata,
      package: prep.package.clone(),
    });
    let rsp = self
      .call::<_, schema::run_deployment_creation::ResponseData>(q)
      .await?
      .check_service_error()?;
    let rsp = rsp
      .data
      .as_ref()
      .map(|x| &x.create_deployment)
      .ok_or_else(|| anyhow::anyhow!("missing data in result"))?;

    {
      let mut stdout = StandardStream::stdout(ColorChoice::Auto);
      stdout.set_color(ColorSpec::new().set_bold(true).set_fg(Some(Color::Cyan)))?;
      writeln!(&mut stdout, "Created deployment {}.", rsp.id)?;
      stdout.reset()?;
    }
    println!("Preview: {}", rsp.url);
    println!("Visit the dashboard to promote this deployment to live.");
    Ok(())
  }
}

pub trait GqlResponseExt: Sized {
  fn check_service_error(self) -> anyhow::Result<Self>;
}

impl<D> GqlResponseExt for graphql_client::Response<D> {
  fn check_service_error(self) -> anyhow::Result<Self> {
    let errors = self.errors.as_ref().map(|x| x.as_slice()).unwrap_or(&[]);
    if !errors.is_empty() {
      anyhow::bail!("service returned error: {}", errors[0].message);
    }
    Ok(self)
  }
}
