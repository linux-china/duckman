use crate::duckman_config::DuckmanConfig;
use colored::Colorize;
use futures_util::StreamExt;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const CORE_EXTENSIONS_CSV: &str = include_str!("resources/core_extensions.csv");
const COMMUNITY_EXTENSIONS_CSV: &str = include_str!("resources/community_extensions.csv");

const DUCKDB_CORE_EXTENSIONS: [&str; 28] = [
    "autocomplete",
    "avro",
    "aws",
    "azure",
    "delta",
    "ducklake",
    "encodings",
    "excel",
    "fts",
    "httpfs",
    "iceberg",
    "icu",
    "inet",
    "jemalloc",
    "json",
    "lance",
    "motherduck",
    "mysql",
    "parquet",
    "postgres",
    "spatial",
    "sqlite",
    "tpcds",
    "tpch",
    "unity_catalog",
    "ui",
    "vortex",
    "vss",
];

// Platform identifier used in the extensions URL and install path
fn platform_ext_id() -> &'static str {
    if cfg!(target_os = "macos") && cfg!(target_arch = "aarch64") {
        "osx_arm64"
    } else if cfg!(target_os = "macos") {
        "osx_amd64"
    } else if cfg!(target_os = "linux") && cfg!(target_arch = "aarch64") {
        "linux_arm64"
    } else if cfg!(target_os = "linux") {
        "linux_amd64"
    } else if cfg!(target_os = "windows") {
        "windows_amd64"
    } else {
        "linux_amd64"
    }
}

fn find_duckdb_binary(duckdb_version: Option<&str>) -> anyhow::Result<PathBuf> {
    // find specified version
    if let Some(duckdb_version) = duckdb_version {
        return Ok(DuckmanConfig::version_binary(duckdb_version));
    }
    let config = DuckmanConfig::load()?;
    if !config.default.is_none() {
        let path = DuckmanConfig::version_binary(&config.default.clone().unwrap());
        if path.exists() {
            return Ok(path);
        }
    }
    for version in config.installed_versions() {
        let path = DuckmanConfig::version_binary(&version);
        if path.exists() {
            return Ok(path);
        }
    }
    // Fall back to duckdb in PATH
    Ok(PathBuf::from("duckdb"))
}

fn get_default_version() -> anyhow::Result<String> {
    let config = DuckmanConfig::load()?;
    if config.default.is_some() {
        return Ok(config.default.unwrap());
    }
    let versions = config.installed_versions();
    if let Some(v) = versions.into_iter().next() {
        return Ok(v);
    }
    anyhow::bail!("No DuckDB version installed. Run `duckman install <version>` first.")
}

fn extensions_dir(version: &str) -> PathBuf {
    DuckmanConfig::home_dir()
        .join("extensions")
        .join(version)
        .join(platform_ext_id())
}

fn extension_path(version: &str, name: &str) -> PathBuf {
    extensions_dir(version).join(format!("{}.duckdb_extension", name))
}

// ── list ─────────────────────────────────────────────────────────────────────

pub fn list_extensions(remote: bool) -> anyhow::Result<()> {
    if remote {
        list_remote_extensions()
    } else {
        list_local_extensions()
    }
}

fn list_local_extensions() -> anyhow::Result<()> {
    let duckdb = find_duckdb_binary(None)?;
    let output = Command::new(&duckdb)
        .args([
            "-c",
            "select extension_name, installed, description FROM duckdb_extensions() where installed = true",
        ])
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                print!("{}", String::from_utf8_lossy(&out.stdout));
            } else {
                eprintln!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => anyhow::bail!("Failed to run duckdb ({}): {}", duckdb.display(), e),
    }
    Ok(())
}

fn list_remote_extensions() -> anyhow::Result<()> {
    // ── Core extensions ───────────────────────────────────────────────────────
    println!("{}", "Core extensions:".bold());
    let mut rdr = csv::Reader::from_reader(CORE_EXTENSIONS_CSV.as_bytes());
    for rec in rdr.records().filter_map(|r| r.ok()) {
        let name = &rec[0];
        let desc = &rec[1];
        let tier = rec.get(3).unwrap_or("").trim();
        let tier_label = match tier {
            "Primary" => format!("[{}]", "primary".cyan()),
            "Secondary" => format!("[{}]", "secondary".dimmed()),
            _ => String::new(),
        };
        println!("  {:<20} {} {}", name.green(), tier_label, desc.dimmed());
    }

    // ── Community extensions ──────────────────────────────────────────────────
    println!();
    println!("{}", "Community extensions:".bold());
    let mut rdr = csv::Reader::from_reader(COMMUNITY_EXTENSIONS_CSV.as_bytes());
    for rec in rdr.records().filter_map(|r| r.ok()) {
        let name = &rec[0];
        let desc = &rec[1];
        println!("  {:<20} {}", name.green(), desc.dimmed());
    }

    Ok(())
}

// ── install ───────────────────────────────────────────────────────────────────

pub async fn install_extension(duckdb_version: Option<&str>, name: &str) -> anyhow::Result<()> {
    let duckdb = find_duckdb_binary(duckdb_version)?;
    let sql = if DUCKDB_CORE_EXTENSIONS.contains(&name) {
        format!("install {}", name)
    } else {
        format!("install {} from community", name)
    };
    let output = Command::new(&duckdb).args(["-c", &sql]).output();
    match output {
        Ok(out) => {
            if out.status.success() {
                let output = String::from_utf8_lossy(&out.stdout);
                if (!output.trim().is_empty()) {
                    println!("{}", output);
                } else {
                    println!("Installed extension {}", name.green(),);
                }
            } else {
                eprintln!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => anyhow::bail!("Failed to install extension ({}): {}", name, e),
    }
    Ok(())
}

// ── uninstall ─────────────────────────────────────────────────────────────────

pub fn uninstall_extension(name: &str) -> anyhow::Result<()> {
    let version = get_default_version()?;
    let path = extension_path(&version, name);
    if !path.exists() {
        anyhow::bail!(
            "Extension '{}' is not installed (looked at {})",
            name,
            path.display()
        );
    }
    fs::remove_file(&path)?;
    println!("Uninstalled extension {}.", name.green());
    Ok(())
}

// ── update ────────────────────────────────────────────────────────────────────

pub fn update_extensions() -> anyhow::Result<()> {
    let duckdb = find_duckdb_binary(None)?;
    let output = Command::new(&duckdb)
        .args(["-c", "UPDATE EXTENSIONS"])
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                print!("{}", String::from_utf8_lossy(&out.stdout));
            } else {
                eprintln!("{}", String::from_utf8_lossy(&out.stderr));
            }
        }
        Err(e) => anyhow::bail!("Failed to run duckdb ({}): {}", duckdb.display(), e),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    #[tokio::test]
    async fn test_install_extension() -> TestResult {
        let ext_name = "shellfs";
        install_extension(None, ext_name).await?;
        Ok(())
    }
}
