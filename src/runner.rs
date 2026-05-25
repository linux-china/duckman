use crate::duckman_config::{DuckmanConfig, inject_profile};
use std::collections::HashMap;
use std::env;

pub fn duckdb_execute(
    config: &DuckmanConfig,
    duckdb_version: &str,
    duckdb_profile: &Option<String>,
    extra_args: Vec<String>,
) -> anyhow::Result<()> {
    let binary = DuckmanConfig::version_binary(&duckdb_version);
    if !binary.exists() {
        anyhow::bail!(
            "DuckDB {} is not installed. Run `duckman install {}` first.",
            duckdb_version,
            duckdb_version
        );
    }
    // private key
    let private_key = &config.get_private_key();
    // environment variables
    let mut new_env: HashMap<String, String> = env::vars().collect();
    let mut new_extra_args = vec![];
    new_extra_args.extend(extra_args);
    // profiles
    let profiles = config.get_profiles();
    if !profiles.is_empty() {
        // default profile check
        if let Some(default_profile) = profiles.get("default") {
            //println!("Using default profile: {}", "default");
            inject_profile(
                duckdb_version,
                default_profile,
                &mut new_extra_args,
                &mut new_env,
                private_key,
            )
        }
        if let Some(profile_name) = duckdb_profile {
            if profile_name != "default" {
                if let Some(profile) = profiles.get(profile_name) {
                    //println!("Using profile: {}", profile_name);
                    inject_profile(
                        duckdb_version,
                        profile,
                        &mut new_extra_args,
                        &mut new_env,
                        private_key,
                    )
                }
            }
        }
    }
    // On Unix: replace this process with duckdb — stdin/stdout/stderr are
    // inherited automatically, so pipes work transparently.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&binary)
            .args(&new_extra_args)
            .envs(&new_env)
            .exec();
        anyhow::bail!("Failed to exec {}: {}", binary.display(), err);
    }

    // On Windows: spawn and forward the exit code.
    #[cfg(not(unix))]
    {
        let status = std::process::Command::new(&binary)
            .args(&new_extra_args)
            .envs(&new_env)
            .status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}
