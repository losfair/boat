use graphql_client::GraphQLQuery;

pub type DateTime = String;

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "schema/api.graphql",
  query_path = "schema/query.graphql"
)]
pub struct RunDeploymentCreation;

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "schema/api.graphql",
  query_path = "schema/query.graphql"
)]
pub struct RunDeploymentPreparation;

#[derive(GraphQLQuery)]
#[graphql(
  schema_path = "schema/api.graphql",
  query_path = "schema/query.graphql"
)]
pub struct RunDeploymentList;
