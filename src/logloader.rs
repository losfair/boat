use graphql_client::{GraphQLQuery, Response};

use crate::{
  cursor::ServiceCursor,
  schema,
  service::{GqlResponseExt, Service},
};
use serde::Deserialize;

pub struct LogLoader<'a> {
  service: &'a Service,
  cursor: ServiceCursor<String>,
  app_id: String,
  deployment_id: Option<String>,
}

#[derive(Deserialize)]
pub struct GenericLog {
  pub ts: i64,
  pub request_id: String,
  pub seq: i64,
  pub message: String,
}

#[derive(Deserialize)]
struct GenericLogList {
  data: Vec<GenericLog>,
  cursor: Option<String>,
}

impl<'a> LogLoader<'a> {
  pub fn new(service: &'a Service, app_id: &str, deployment_id: Option<&str>) -> Self {
    Self {
      service,
      cursor: ServiceCursor::Initial,
      app_id: app_id.to_string(),
      deployment_id: deployment_id.map(|s| s.to_string()),
    }
  }

  pub async fn load_logs(&mut self, page_size: u32) -> anyhow::Result<Vec<GenericLog>> {
    if matches!(self.cursor, ServiceCursor::End) {
      return Ok(vec![]);
    }

    let log_list = if let Some(deployment_id) = &self.deployment_id {
      self
        .query_logs_for_deployment(deployment_id, page_size, &self.cursor)
        .await?
    } else {
      self
        .query_logs_for_app(&self.app_id, page_size, &self.cursor)
        .await?
    };

    self.cursor = if let Some(x) = log_list.cursor {
      ServiceCursor::Next(x)
    } else {
      ServiceCursor::End
    };
    Ok(log_list.data)
  }

  async fn query_logs_for_app(
    &self,
    app_id: &str,
    first: u32,
    before: &ServiceCursor<String>,
  ) -> anyhow::Result<GenericLogList> {
    let query = schema::GetAppLogs::build_query(schema::get_app_logs::Variables {
      id: app_id.to_string(),
      first: Some(first as i64),
      before: before.as_request_cursor_ref().cloned(),
    });
    let rsp: Response<schema::get_app_logs::ResponseData> =
      self.service.call(query).await?.check_service_error()?;

    let data = rsp
      .data
      .and_then(|x| x.app)
      .and_then(|x| x.current_deployment.map(|x| x.logs))
      .ok_or_else(|| anyhow::anyhow!("missing data"))?;

    Ok(serde_json::from_str(&serde_json::to_string(&data)?)?)
  }

  async fn query_logs_for_deployment(
    &self,
    deployment_id: &str,
    first: u32,
    before: &ServiceCursor<String>,
  ) -> anyhow::Result<GenericLogList> {
    let query = schema::GetDeploymentLogs::build_query(schema::get_deployment_logs::Variables {
      id: deployment_id.to_string(),
      first: Some(first as i64),
      before: before.as_request_cursor_ref().cloned(),
    });
    let rsp: Response<schema::get_deployment_logs::ResponseData> =
      self.service.call(query).await?.check_service_error()?;

    let data = rsp
      .data
      .and_then(|x| x.deployment.map(|x| x.logs))
      .ok_or_else(|| anyhow::anyhow!("missing data"))?;

    Ok(serde_json::from_str(&serde_json::to_string(&data)?)?)
  }
}
