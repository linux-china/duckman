use crate::duckman_config::DuckmanConfig;
use crate::runner::duckdb_execute;
use anyhow::bail;
use std::env;
use std::ffi::OsString;

mod duckman_config;
mod runner;

fn main() -> anyhow::Result<()> {
    let mut raw_args: Vec<OsString> = env::args_os().collect();
    // get shim command name
    let shim_command = raw_args[0].clone().to_str().unwrap().to_owned();
    let config = DuckmanConfig::load().unwrap();
    let duckdb_version = env::var("DUCKDB_VERSION")
        .ok()
        .unwrap_or_else(|| config.default.clone().unwrap_or("".to_string()));
    let duckdb_profile = env::var("DUCKDB_PROFILE").ok();

    if duckdb_version.is_empty() {
        bail!(
            "No DuckDB version specified and no default set. \
             Run `duckman install <version>` first."
        );
    }
    let extra_args: Vec<String> = raw_args
        .iter()
        .skip(1)
        .map(|os| os.to_str().unwrap().to_owned())
        .collect();
    duckdb_execute(&config, &duckdb_version, &duckdb_profile, extra_args)
}
