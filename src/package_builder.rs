use std::{collections::BTreeMap, path::Path, process::Command};

use crate::config::{AppConfig, AppSpec};
use tempdir::TempDir;

pub fn build_package(
  spec_path: &Path,
  spec: &AppSpec,
  config: &AppConfig,
) -> anyhow::Result<Vec<u8>> {
  let spec_dir = spec_path
    .parent()
    .ok_or_else(|| anyhow::anyhow!("cannot resolve spec parent dir"))?
    .to_path_buf();

  if let Some(build) = &spec.build {
    let mut newenv: BTreeMap<String, String> = std::env::vars().collect();
    for (k, v) in &config.env {
      newenv.insert(format!("BLUEBOAT_{}", k.get_ref()), v.to_string());
    }

    let status = Command::new("sh")
      .envs(newenv)
      .current_dir(&spec_dir)
      .args(["-c", build.as_str()])
      .status()?;
    if !status.success() {
      anyhow::bail!("build failed: {}", status.code().unwrap_or(1));
    }
  }

  let td = TempDir::new("bbcli-deploy")?;
  if let Some(d) = &spec._static {
    let status = {
      #[cfg(target_os = "macos")]
      {
        let mut d = spec_dir.join(d).canonicalize()?;
        d.push("");
        Command::new("cp")
          .args([Path::new("-r"), &d, td.path()])
          .status()?
      }
      #[cfg(not(target_os = "macos"))]
      {
        let d = spec_dir.join(d).canonicalize()?;
        Command::new("cp")
          .args([Path::new("-rT"), &d, td.path()])
          .status()?
      }
    };
    if !status.success() {
      anyhow::bail!("copy static failed: {}", status.code().unwrap_or(1));
    }
  }

  let mut artifact_target_path = td.path().to_path_buf();
  artifact_target_path.push("index.js");
  let artifact_source_path = spec_dir.join(&spec.artifact).canonicalize()?;
  std::fs::copy(&artifact_source_path, &artifact_target_path)?;

  let mut tar_builder = tar::Builder::new(Vec::new());
  tar_builder.append_dir_all(".", td.path())?;
  let image = tar_builder.into_inner()?;
  log::info!("Image size is {} bytes.", image.len());

  Ok(image)
}
