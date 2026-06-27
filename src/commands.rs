use crate::duckman_app::build_duckman_app;
use crate::duckman_config::{DuckmanConfig, normalize_duckdb_version};
use crate::github;
use crate::runner::duckdb_execute;
use clap::ArgMatches;
use clap_complete::Shell::{Bash, Fish, PowerShell, Zsh};
use clap_complete::{Shell, generate};
use colored::Colorize;
use futures_util::StreamExt;
use std::io::stdout;
use std::{env, fs};

const DUCKDB_VERSIONS_CSV: &str = include_str!("resources/duckdb_versions.csv");

struct DuckdbVersionRecord {
    version: String,
    date: String,
}

fn load_version_list() -> Vec<DuckdbVersionRecord> {
    let mut rdr = csv::Reader::from_reader(DUCKDB_VERSIONS_CSV.as_bytes());
    rdr.records()
        .filter_map(|r| r.ok())
        .map(|r| DuckdbVersionRecord {
            version: r[0].to_string(),
            date: r[1].to_string(),
        })
        .collect()
}

pub fn count_versions() -> anyhow::Result<()> {
    let versions = DuckmanConfig::installed_versions();
    println!(
        "Installed DuckDB versions: {} 🦆",
        versions.len().to_string().green()
    );
    for version in &versions {
        let binary = DuckmanConfig::version_binary(version);
        let ext_count = if binary.exists() {
            let output = std::process::Command::new(&binary)
                .args([
                    "-csv",
                    "-c",
                    "SELECT count(*) FROM duckdb_extensions() WHERE installed = true",
                ])
                .output();
            match output {
                Ok(out) if out.status.success() => {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    stdout
                        .lines()
                        .nth(1)
                        .and_then(|l| l.trim().parse::<usize>().ok())
                        .unwrap_or(0)
                }
                _ => 0,
            }
        } else {
            0
        };
        println!(
            "  {}  extensions: {}",
            version.green(),
            ext_count.to_string().cyan()
        );
    }
    Ok(())
}

pub async fn list_versions(local: bool, remote: bool) -> anyhow::Result<()> {
    // Default: show local if no flags given
    let show_local = local || (!local && !remote);
    let show_remote = remote;

    if show_local {
        let config = DuckmanConfig::load()?;
        let versions = DuckmanConfig::installed_versions();
        println!("{}", "Installed versions:".bold());
        if versions.is_empty() {
            println!("  (none)");
        } else {
            let default_version = config.default.unwrap_or("".to_string());
            for v in &versions {
                if v == &default_version {
                    println!("  {} {}", v.green(), "(default)".dimmed());
                } else {
                    println!("  {}", v);
                }
            }
        }
    }

    if show_remote {
        println!("{}", "Available versions:".bold());
        for entry in load_version_list() {
            let installed = DuckmanConfig::is_duckdb_installed(&entry.version);
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

fn version_from_path_binary(binary_path: &std::path::Path) -> anyhow::Result<String> {
    let output = std::process::Command::new(binary_path)
        .arg("--version")
        .output()?;
    if !output.status.success() {
        anyhow::bail!("Failed to run {} --version", binary_path.display());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let version = stdout
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("Empty output from {} --version", binary_path.display()))?
        .to_string();
    Ok(version)
}

async fn install_from_path(src: &str) -> anyhow::Result<()> {
    let src_path = std::path::Path::new(src);
    if !src_path.exists() {
        anyhow::bail!("File not found: {}", src);
    }
    let version = version_from_path_binary(src_path)?;
    let version = normalize_duckdb_version(&version);
    let mut config = DuckmanConfig::load()?;

    if DuckmanConfig::is_duckdb_installed(&version) {
        println!("DuckDB {} is already installed.", version.green());
        return Ok(());
    }

    let version_dir = DuckmanConfig::version_dir(&version);
    fs::create_dir_all(&version_dir)?;
    let binary_path = DuckmanConfig::version_binary(&version);
    fs::copy(src_path, &binary_path)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&binary_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&binary_path, perms)?;
    }

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

pub async fn install_version(version: &str) -> anyhow::Result<()> {
    if version.contains('/') || version.contains('\\') {
        return install_from_path(version).await;
    }
    if version == "system" {
        let path =
            which::which("duckdb").map_err(|_| anyhow::anyhow!("duckdb not found in PATH"))?;
        return install_from_path(path.to_str().unwrap()).await;
    }
    let version = normalize_duckdb_version(version);
    let mut config = DuckmanConfig::load()?;

    if DuckmanConfig::is_duckdb_installed(&version) {
        println!("DuckDB {} is already installed.", version.green());
        return Ok(());
    }

    // Fetch release metadata from GitHub
    github::download_duckdb(&version).await?;

    // Set as default if none is set
    if config.default.is_none() {
        config.set_default(&version);
        println!("Set {} as the default version.", version.green());
    }
    config.save()?;

    let binary_path = DuckmanConfig::version_binary(&version);

    println!(
        "Installed DuckDB {} -> {}",
        version.green(),
        binary_path.display()
    );
    Ok(())
}

pub async fn uninstall_version(version: &str) -> anyhow::Result<()> {
    let version = normalize_duckdb_version(version);
    let mut config = DuckmanConfig::load()?;

    if !DuckmanConfig::is_duckdb_installed(&version) {
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

pub async fn run_duckdb(profile: Option<&str>, extra_args: Vec<String>) -> anyhow::Result<()> {
    let config = DuckmanConfig::load()?;

    // Version resolution: explicit arg > DUCKDB_VERSION env > config default
    let duckdb_profile = profile
        .map(|p| p.to_string())
        .or_else(|| env::var("DUCKDB_PROFILE").ok());
    let duckdb_version = config.get_duckdb_version(&duckdb_profile);

    if duckdb_version.is_none() {
        anyhow::bail!(
            "No DuckDB version specified and no default set. \
             Run `duckman install <version>` first."
        );
    }
    duckdb_execute(
        &config,
        &duckdb_version.unwrap(),
        &duckdb_profile,
        extra_args,
    )
    .await
}

pub fn which_duckdb(version: Option<&str>) -> anyhow::Result<()> {
    let config = DuckmanConfig::load()?;
    let resolved = version
        .map(normalize_duckdb_version)
        .or_else(|| config.get_duckdb_version(&None));

    let resolved = match resolved {
        Some(v) => v,
        None => anyhow::bail!(
            "No DuckDB version specified and no default set. \
             Run `duckman install <version>` first."
        ),
    };

    let binary = DuckmanConfig::version_binary(&resolved);
    if !binary.exists() {
        anyhow::bail!(
            "DuckDB {} is not installed. Run `duckman install {}` first.",
            resolved,
            resolved
        );
    }

    println!("{}", binary.display());
    Ok(())
}

pub fn set_default_version(version: &str) -> anyhow::Result<()> {
    let version = normalize_duckdb_version(version);
    if !DuckmanConfig::is_duckdb_installed(&version) {
        anyhow::bail!(
            "DuckDB {} is not installed. Run `duckman install {}` first.",
            version,
            version
        );
    }
    let mut config = DuckmanConfig::load_global()?;
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
