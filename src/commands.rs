use crate::duckman_app::build_duckman_app;
use crate::duckman_config::{DuckmanConfig, Profile};
use crate::github;
use clap::ArgMatches;
use clap_complete::Shell::{Bash, Fish, PowerShell, Zsh};
use clap_complete::{Shell, generate};
use colored::Colorize;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::io::{Cursor, stdout};
use std::{env, fs};

const DUCKDB_VERSIONS_CSV: &str = include_str!("resources/duckdb_versions.csv");

struct VersionEntry {
    version: String,
    date: String,
}

fn load_version_list() -> Vec<VersionEntry> {
    let mut rdr = csv::Reader::from_reader(DUCKDB_VERSIONS_CSV.as_bytes());
    rdr.records()
        .filter_map(|r| r.ok())
        .map(|r| VersionEntry {
            version: r[0].to_string(),
            date: r[1].to_string(),
        })
        .collect()
}

fn platform_asset_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "duckdb_cli-osx-universal.zip"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "duckdb_cli-linux-aarch64.zip"
    } else if cfg!(target_os = "linux") {
        "duckdb_cli-linux-amd64.zip"
    } else if cfg!(target_os = "windows") {
        "duckdb_cli-windows-amd64.zip"
    } else {
        "duckdb_cli-linux-amd64.zip"
    }
}

fn normalize_version(version: &str) -> String {
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    }
}

pub async fn list_versions(local: bool, remote: bool) -> anyhow::Result<()> {
    // Default: show local if no flags given
    let show_local = local || (!local && !remote);
    let show_remote = remote;

    if show_local {
        let config = DuckmanConfig::load()?;
        let versions = config.installed_versions();
        println!("{}", "Installed versions:".bold());
        if versions.is_empty() {
            println!("  (none)");
        } else {
            let default_version = config.default.unwrap_or("".to_string());
            for v in &versions {
                if *v == default_version {
                    println!("  {} {}", v.green(), "(default)".dimmed());
                } else {
                    println!("  {}", v);
                }
            }
        }
    }

    if show_remote {
        let config = DuckmanConfig::load()?;
        println!("{}", "Available versions:".bold());
        for entry in load_version_list() {
            let installed = config.is_installed(&entry.version);
            if installed {
                println!(
                    "  {}  {}  {}",
                    entry.version.green(),
                    entry.date.dimmed(),
                    "(installed)".dimmed()
                );
            } else {
                println!("  {}  {}", entry.version, entry.date.dimmed());
            }
        }
    }

    Ok(())
}

pub async fn install_version(version: &str) -> anyhow::Result<()> {
    let version = normalize_version(version);
    let mut config = DuckmanConfig::load()?;

    if config.is_installed(&version) {
        println!("DuckDB {} is already installed.", version.green());
        return Ok(());
    }

    // Fetch release metadata from GitHub
    println!("Fetching release info for {}...", version);
    let release = github::fetch_release(&version).await?;
    let release = match release {
        Some(r) => r,
        None => {
            anyhow::bail!("Version {} not found on GitHub releases.", version);
        }
    };

    let asset_name = platform_asset_name();
    let asset = match release.find_asset(asset_name) {
        Some(a) => a,
        None => {
            anyhow::bail!(
                "No binary found for your platform (expected asset: {}). \
                 Available assets: {}",
                asset_name,
                release
                    .assets
                    .iter()
                    .map(|a| a.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    };

    let pb = ProgressBar::new(asset.size);
    pb.set_style(
        ProgressStyle::default_bar()
            .template(
                "{spinner:.green} Downloading [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({binary_bytes_per_sec}, {eta})",
            )?
            .progress_chars("=>-"),
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&asset.browser_download_url)
        .header("User-Agent", "Duckman/0.1.0")
        .send()
        .await?;

    let mut stream = response.bytes_stream();
    let mut buf: Vec<u8> = Vec::with_capacity(asset.size as usize);
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        pb.inc(chunk.len() as u64);
        buf.extend_from_slice(&chunk);
    }
    pb.finish_and_clear();
    let bytes = buf;

    // Extract binary from zip
    println!("Extracting...");
    let cursor = Cursor::new(bytes.to_vec());
    let mut archive = zip::ZipArchive::new(cursor)?;

    let version_dir = DuckmanConfig::version_dir(&version);
    fs::create_dir_all(&version_dir)?;
    let binary_path = DuckmanConfig::version_binary(&version);

    let mut found = false;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_owned();
        // Match "duckdb" or "duckdb.exe" anywhere in the archive path
        let base = name.split('/').last().unwrap_or(&name);
        if base == "duckdb" || base == "duckdb.exe" {
            let mut out = fs::File::create(&binary_path)?;
            std::io::copy(&mut entry, &mut out)?;
            found = true;
            break;
        }
    }

    if !found {
        fs::remove_dir_all(&version_dir)?;
        anyhow::bail!("Could not find duckdb binary inside the downloaded archive.");
    }

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms)?;
    }

    // Set as default if none is set
    if config.default.is_none() {
        config.set_default(&version);
        println!("Set {} as the default version.", version.green());
    }
    config.save()?;

    println!(
        "Installed DuckDB {} -> {}",
        version.green(),
        binary_path.display()
    );
    Ok(())
}

pub async fn uninstall_version(version: &str) -> anyhow::Result<()> {
    let version = normalize_version(version);
    let mut config = DuckmanConfig::load()?;

    if !config.is_installed(&version) {
        anyhow::bail!("DuckDB {} is not installed.", version);
    }

    let version_dir = DuckmanConfig::version_dir(&version);
    fs::remove_dir_all(&version_dir)?;

    // Clear default if it pointed to this version
    if config.default.as_ref() == Some(&version) {
        config.default = None;
    }
    config.save()?;

    println!("Uninstalled DuckDB {}.", version.green());
    Ok(())
}

pub fn run_duckdb(
    version: Option<&str>,
    profile: Option<&str>,
    extra_args: Vec<String>,
) -> anyhow::Result<()> {
    let config = DuckmanConfig::load()?;

    // Version resolution: explicit arg > DUCKDB_VERSION env > config default
    let duckdb_version = version
        .map(normalize_version)
        .or_else(|| env::var("DUCKDB_VERSION").ok())
        .unwrap_or_else(|| config.default.clone().unwrap_or("".to_string()));

    // Version resolution: explicit arg > DUCKDB_VERSION env > config default
    let duckdb_profile = profile
        .map(|p| p.to_string())
        .or_else(|| env::var("DUCKDB_PROFILE").ok());

    if duckdb_version.is_empty() {
        anyhow::bail!(
            "No DuckDB version specified and no default set. \
             Run `duckman install <version>` first."
        );
    }

    let binary = DuckmanConfig::version_binary(&duckdb_version);
    if !binary.exists() {
        anyhow::bail!(
            "DuckDB {} is not installed. Run `duckman install {}` first.",
            duckdb_version,
            duckdb_version
        );
    }
    // environment variables
    let mut new_env: HashMap<String, String> = env::vars().collect();
    let mut new_extra_args = vec![];
    new_extra_args.extend(extra_args);
    // profiles
    let profiles = config.get_profiles();
    if !profiles.is_empty() {
        // default profile check
        if let Some(default_profile) = profiles.get("default") {
            println!("Using default profile: {}", "default");
            inject_profile(default_profile, &mut new_extra_args, &mut new_env)
        }
        if let Some(profile_name) = duckdb_profile {
            if let Some(profile) = profiles.get(&profile_name) {
                println!("Using profile: {}", profile_name);
                inject_profile(profile, &mut new_extra_args, &mut new_env)
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
            .args(&extra_args)
            .envs(&new_extra_args)
            .status()?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

fn inject_profile(
    profile: &Profile,
    args: &mut Vec<String>,
    new_env: &mut HashMap<String, String>,
) {
    // environment variable
    if !profile.environment.is_empty() {
        new_env.extend(
            profile
                .environment
                .clone()
                .iter()
                .map(|(k, v)| (k.to_uppercase(), v.to_string())),
        );
    }
}

pub fn set_default_version(version: &str) -> anyhow::Result<()> {
    let version = normalize_version(version);
    let mut config = DuckmanConfig::load()?;

    if !config.is_installed(&version) {
        anyhow::bail!(
            "DuckDB {} is not installed. Run `duckman install {}` first.",
            version,
            version
        );
    }

    config.set_default(&version);
    config.save()?;
    println!("Default DuckDB version set to {}.", version.green());
    Ok(())
}

pub fn completion_command(command_matches: &ArgMatches) {
    let shell_name = command_matches
        .get_one::<String>("shell")
        .map(|s| s.to_string())
        .unwrap_or_else(|| Shell::from_env().unwrap_or(Bash).to_string())
        .to_lowercase();
    let mut cmd = build_duckman_app();
    if shell_name == "bash" {
        generate(Bash, &mut cmd, "dotenvx", &mut stdout());
    } else if shell_name == "zsh" {
        generate(Zsh, &mut cmd, "dotenvx", &mut stdout());
    } else if shell_name == "fish" {
        generate(Fish, &mut cmd, "dotenvx", &mut stdout());
    } else if shell_name == "powershell" || shell_name == "pwsh" {
        generate(PowerShell, &mut cmd, "dotenvx", &mut stdout());
    } else {
        eprintln!(
            "Unsupported shell: {shell_name}. Supported shells are bash/zsh/fish/powershell."
        );
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    #[tokio::test]
    async fn test_install_version() -> TestResult {
        install_version("v1.3.2").await?;
        Ok(())
    }
}
