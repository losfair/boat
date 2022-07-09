use boatctl::{
  config_loader,
  metadata::AppMetadata,
  package_builder::build_package,
  schema::{self, RunDeploymentList},
  service::{GqlResponseExt, Service},
};
use graphql_client::GraphQLQuery;
use structopt::StructOpt;
use tabled::{Style, Table, Tabled};

#[derive(Debug, StructOpt)]
#[structopt(name = "boat", about = "Blueboat Cloud CLI")]
struct Opt {
  /// Lighthouse service endpoint.
  #[structopt(
    long,
    default_value = "https://magic.blueboat.io/graphql",
    env = "BOAT_ENDPOINT"
  )]
  endpoint: String,

  /// Path to API credentials.
  #[structopt(long, env = "BOAT_CREDENTIALS")]
  credentials: Option<String>,

  /// Path to app specification.
  #[structopt(long, default_value = "Boat.spec.toml", env = "BOAT_SPEC")]
  spec: String,

  /// Path to app config.
  #[structopt(long, default_value = "Boat.toml", env = "BOAT_CONFIG")]
  config: String,

  #[structopt(subcommand)]
  cmd: Cmd,
}

#[derive(Debug, StructOpt)]
enum Cmd {
  /// Create deployme&nt.
  Deploy,

  /// List deployments.
  List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  pretty_env_logger::init_timed();

  let opt = Opt::from_args();

  let service = Service::new(&opt.endpoint, &opt.credentials)?;
  let ((spec_path, spec), (_config_path, config)) =
    match config_loader::load_from_file(&opt.spec, &opt.config) {
      Ok(x) => x,
      Err(e) => {
        eprintln!("{:?}", e);
        std::process::exit(1);
      }
    };
  match &opt.cmd {
    Cmd::List => {
      let q = RunDeploymentList::build_query(schema::run_deployment_list::Variables {
        app_id: config.id.clone(),
        first: Some(100),
        offset: None,
      });
      let rsp = service
        .call::<_, schema::run_deployment_list::ResponseData>(q)
        .await?
        .check_service_error()?;
      let x = rsp
        .data
        .as_ref()
        .map(|x| x.list_deployment.as_slice())
        .unwrap_or_default();
      let table_data = x
        .iter()
        .map(|x| DeploymentEntry {
          id: &x.id,
          created_at: &x.created_at,
          live: if x.live { "✔" } else { "" },
        })
        .collect::<Vec<_>>();
      let table = Table::new(&table_data).with(Style::psql());
      println!("{}", table);
    }
    Cmd::Deploy => {
      let package = build_package(&spec_path, &spec, &config)
        .map_err(|e| e.context("failed to build package"))?;
      let metadata = AppMetadata::from_config(&config);
      service.deploy(&config.id, &metadata, &package).await?;
    }
  }
  Ok(())
}

#[derive(Tabled)]
struct DeploymentEntry<'a> {
  #[tabled(rename = "ID")]
  id: &'a str,
  #[tabled(rename = "Created at")]
  created_at: &'a str,
  #[tabled(rename = "Live")]
  live: &'static str,
}
