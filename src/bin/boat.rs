use boatctl::{
  config_loader,
  schema::{self, RunDeploymentList},
  service::Service,
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
    default_value = "https://cloud.blueboat.io/graphql",
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
  /// Create deployment.
  Deploy,

  /// List deployments.
  List,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  pretty_env_logger::init_timed();

  let opt = Opt::from_args();

  let service = Service::new(&opt.endpoint, &opt.credentials)?;
  let (_spec, config) = match config_loader::load_from_file(&opt.spec, &opt.config) {
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
        .await?;
      let errors = rsp.errors.as_ref().map(|x| x.as_slice()).unwrap_or(&[]);
      if !errors.is_empty() {
        anyhow::bail!("service returned error: {}", errors[0].message);
      }
      let x = rsp
        .data
        .as_ref()
        .and_then(|x| x.list_deployment.as_ref())
        .map(|x| x.iter().filter_map(|x| x.as_ref()).collect::<Vec<_>>())
        .unwrap_or_default();
      let table_data = x
        .iter()
        .map(|x| DeploymentEntry {
          id: &x.id,
          created_at: &x.created_at,
        })
        .collect::<Vec<_>>();
      let table = Table::new(&table_data).with(Style::psql());
      println!("{}", table);
    }
    Cmd::Deploy => {
      todo!()
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
}
