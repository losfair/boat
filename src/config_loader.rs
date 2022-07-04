use std::collections::HashMap;

use crate::config::{AppConfig, AppSpec};
use miette::{Diagnostic, IntoDiagnostic, NamedSource, SourceOffset, SourceSpan};
use regex::Regex;
use serde::Deserialize;
use thiserror::Error;
use toml::Spanned;

#[derive(Error, Debug, Diagnostic)]
#[error("cannot parse config")]
#[diagnostic(code(boatctl::config::parse))]
struct ConfigParseError {
  #[source_code]
  src: NamedSource,

  #[label("This bit here")]
  bad_bit: Option<SourceSpan>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("config validation failed")]
#[diagnostic(code(boatctl::config::validate))]
struct ConfigValidationError {
  #[source_code]
  src: NamedSource,

  #[label("This bit here")]
  bad_bit: Option<SourceSpan>,
}

#[derive(Error, Debug, Diagnostic)]
#[error("duplicate environment variable in spec")]
#[diagnostic(code(boatctl::config::dup_env))]
struct DuplicateSpecEnvError {
  #[source_code]
  src: NamedSource,

  #[label("previous definition")]
  prev_def: SourceSpan,

  #[label("redefined here")]
  redef: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("duplicate environment variable in config")]
#[diagnostic(code(boatctl::config::dup_env))]
struct DuplicateConfigEnvError {
  #[source_code]
  src: NamedSource,

  #[label("previous definition")]
  prev_def: SourceSpan,

  #[label("redefined here")]
  redef: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("undefined environment variable")]
#[diagnostic(code(boatctl::config::undefined_env))]
struct UndefinedEnvError {
  #[source_code]
  src: NamedSource,

  #[label("specified here")]
  def: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("invalid regex for environment variable")]
#[diagnostic(code(boatctl::config::invalid_regex))]
struct InvalidEnvRegexError {
  #[source_code]
  src: NamedSource,

  #[label("specified here")]
  def: SourceSpan,
}

#[derive(Error, Debug, Diagnostic)]
#[error("environment variable value does not match spec")]
#[diagnostic(code(boatctl::config::invalid_env))]
struct EnvDoesNotMatchSpec {
  #[source_code]
  src: NamedSource,

  #[label("defined here")]
  def: SourceSpan,

  #[help]
  help: String,
}

#[derive(Error, Debug, Diagnostic)]
#[error("secret defined as env")]
#[diagnostic(code(boatctl::config::secret_as_env))]
struct SecretDefinedAsEnv {
  #[source_code]
  src: NamedSource,

  #[label("defined as env here")]
  def: SourceSpan,
}

pub fn load(
  (spec_name, spec): (&str, &str),
  (config_name, config): (&str, &str),
) -> miette::Result<(AppSpec, AppConfig)> {
  let parsed_spec: AppSpec = parse_toml(spec_name, spec)?;
  let parsed_config: AppConfig = parse_toml(config_name, config)?;

  validate_spec_no_dup_env_or_secret((spec_name, spec, &parsed_spec))?;
  validate_config_no_dup_env_or_secret((config_name, config, &parsed_config))?;
  validate_env_defined_and_valid(
    (spec_name, spec, &parsed_spec),
    (config_name, config, &parsed_config),
  )?;
  validate_no_secret_defined_as_env(
    (spec_name, spec, &parsed_spec),
    (config_name, config, &parsed_config),
  )?;

  Ok((parsed_spec, parsed_config))
}

pub fn load_from_file(spec_path: &str, config_path: &str) -> miette::Result<(AppSpec, AppConfig)> {
  let spec_path = std::fs::canonicalize(spec_path)
    .into_diagnostic()
    .map_err(|e| e.context("cannot resolve spec path"))?;
  let spec = std::fs::read_to_string(&spec_path)
    .into_diagnostic()
    .map_err(|e| e.context("cannot read spec"))?;

  let config_path = std::fs::canonicalize(config_path)
    .into_diagnostic()
    .map_err(|e| e.context("cannot resolve config path"))?;
  let config = std::fs::read_to_string(&config_path)
    .into_diagnostic()
    .map_err(|e| e.context("cannot read config"))?;

  load(
    (spec_path.to_string_lossy().as_ref(), &spec),
    (config_path.to_string_lossy().as_ref(), &config),
  )
}

fn parse_toml<T: for<'de> Deserialize<'de>>(name: &str, text: &str) -> Result<T, ConfigParseError> {
  toml::from_str(&text).map_err(|e| {
    let loc = e
      .line_col()
      .map(|(line, col)| SourceOffset::from_location(text, line, col));
    ConfigParseError {
      src: NamedSource::new(name, text.to_string()),
      bad_bit: loc.map(|loc| SourceSpan::new(loc, loc)),
    }
  })
}

fn validate_spec_no_dup_env_or_secret(
  (spec_name, spec_text, spec): (&str, &str, &AppSpec),
) -> miette::Result<()> {
  let mut seen: HashMap<String, SourceSpan> = HashMap::new();
  for item in spec.env.iter().chain(spec.secrets.iter()) {
    let spec = item.get_ref().to_env_spec();
    let span = toml_spanned_to_source_span(item);
    if let Some(&prev_span) = seen.get(&spec.key) {
      let (prev_def, redef) = if prev_span.offset() < span.offset() {
        (prev_span, span)
      } else {
        (span, prev_span)
      };
      return Err(
        DuplicateSpecEnvError {
          src: NamedSource::new(spec_name, spec_text.to_string()),
          prev_def,
          redef,
        }
        .into(),
      );
    }
    seen.insert(spec.key.clone(), span);
  }
  Ok(())
}

fn validate_config_no_dup_env_or_secret(
  (config_name, config_text, config): (&str, &str, &AppConfig),
) -> miette::Result<()> {
  let mut seen: HashMap<String, SourceSpan> = HashMap::new();
  for item in config.env.iter().chain(config.secrets.iter()) {
    let key = item.0;
    let span = toml_spanned_to_source_span(key);
    if let Some(&prev_span) = seen.get(key.get_ref()) {
      let (prev_def, redef) = if prev_span.offset() < span.offset() {
        (prev_span, span)
      } else {
        (span, prev_span)
      };
      return Err(
        DuplicateConfigEnvError {
          src: NamedSource::new(config_name, config_text.to_string()),
          prev_def,
          redef,
        }
        .into(),
      );
    }
    seen.insert(key.get_ref().clone(), span);
  }
  Ok(())
}

fn validate_no_secret_defined_as_env(
  (_spec_name, _spec_text, spec): (&str, &str, &AppSpec),
  (config_name, config_text, config): (&str, &str, &AppConfig),
) -> miette::Result<()> {
  for item in spec.secrets.iter() {
    let env_spec = item.get_ref().to_env_spec();
    if let Some((env_key, _)) = config.env.get_key_value(env_spec.key.as_str()) {
      return Err(
        SecretDefinedAsEnv {
          src: NamedSource::new(config_name, config_text.to_string()),
          def: toml_spanned_to_source_span(env_key),
        }
        .into(),
      );
    }
  }
  Ok(())
}

fn validate_env_defined_and_valid(
  (spec_name, spec_text, spec): (&str, &str, &AppSpec),
  (config_name, config_text, config): (&str, &str, &AppConfig),
) -> miette::Result<()> {
  for item in spec.env.iter().chain(spec.secrets.iter()) {
    let env_spec = item.get_ref().to_env_spec();
    let kv = config
      .env
      .get_key_value(env_spec.key.as_str())
      .or_else(|| config.secrets.get_key_value(env_spec.key.as_str()));
    if !env_spec.optional && kv.is_none() {
      return Err(
        UndefinedEnvError {
          src: NamedSource::new(spec_name, spec_text.to_string()),
          def: toml_spanned_to_source_span(item),
        }
        .into(),
      );
    }

    if let Some(regex) = &env_spec.regex {
      let re = match Regex::new(regex) {
        Ok(x) => x,
        Err(_) => {
          return Err(
            InvalidEnvRegexError {
              src: NamedSource::new(spec_name, spec_text.to_string()),
              def: toml_spanned_to_source_span(item),
            }
            .into(),
          )
        }
      };
      if let Some(kv) = kv {
        if !re.is_match(kv.1) {
          return Err(
            EnvDoesNotMatchSpec {
              src: NamedSource::new(config_name, config_text.to_string()),
              def: toml_spanned_to_source_span(kv.0),
              help: format!("regex: {}", regex),
            }
            .into(),
          );
        }
      }
    }
  }
  Ok(())
}

fn toml_spanned_to_source_span<T>(spanned: &Spanned<T>) -> SourceSpan {
  SourceSpan::from(spanned.start()..spanned.end())
}
