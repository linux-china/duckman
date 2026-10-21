use crate::duckman_config::DuckmanConfig;
use crate::runner::duckdb_execute;
use anyhow::bail;
use std::env;
use std::ffi::OsString;

mod duckman_config;
mod runner;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    dotenvx_rs::dotenv().ok();
    let raw_args: Vec<OsString> = env::args_os().collect();
    // get shim command name
    //let shim_command = raw_args[0].clone().to_str().unwrap().to_owned();
    let config = DuckmanConfig::load()?;
    let duckdb_profile = env::var("DUCKDB_PROFILE").ok();
    let duckdb_version = config.get_duckdb_version(None, &duckdb_profile);
    if duckdb_version.is_none() {
        bail!(
            "No DuckDB version specified and no default set. \
             Run `duckman install <version>` first."
        );
    }
    // Guard against self-recursion: the resolved binary must not be the shim itself
    let duckdb_version = duckdb_version.unwrap();
    let binary = DuckmanConfig::version_binary(&duckdb_version);
    if binary.exists() && duckman_config::is_duckman_shim(&binary) {
        bail!(
            "DuckDB {} binary {} points to duckman shim, please reinstall it with `duckman uninstall {} && duckman install {}`",
            duckdb_version,
            binary.display(),
            duckdb_version,
            duckdb_version
        );
    }
    let extra_args: Vec<String> = raw_args
        .iter()
        .skip(1)
        .map(|os| os.to_str().unwrap().to_owned())
        .collect();
    duckdb_execute(
        &config,
        &duckdb_version,
        &duckdb_profile,
        extra_args,
    )
    .await
}
