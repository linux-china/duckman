use crate::duckman_config::{
    COMMUNITY_EXTENSIONS_CSV, CORE_EXTENSIONS_CSV, DUCKDB_CORE_EXTENSIONS, DuckmanConfig,
    normalize_duckdb_version,
};
use colored::Colorize;
use futures_util::StreamExt;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::{env, fs};

// Platform identifier used in the extensions URL and install pat
fn find_duckdb_binary() -> anyhow::Result<PathBuf> {
    // find specified version
    if let Ok(duckdb_version) = env::var("DUCKDB_VERSION") {
        return Ok(DuckmanConfig::version_binary(&duckdb_version));
    }
    // load default from config
    let config = DuckmanConfig::load()?;
    if let Some(duckdb_version) = config.get_duckdb_version(&None) {
        return Ok(DuckmanConfig::version_binary(&duckdb_version));
    }
    // Fall back to duckdb in PATH
    if let Ok(path) = which::which("duckdb") {
        Ok(path)
    } else {
        Err(anyhow::anyhow!("duckdb not found"))
    }
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
    let config = &DuckmanConfig::load()?;
    let duckdb_version = config.get_duckdb_version(&None);
    if let Some(version) = duckdb_version {
        println!("Listing extensions for DuckDB {}", version.green());
    }
    let duckdb = find_duckdb_binary()?;
    let output = Command::new(&duckdb)
        .args([
            "-c",
            "select extension_name, installed, loaded, description FROM duckdb_extensions() where installed = true",
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

pub async fn install_extension(name: &str) -> anyhow::Result<()> {
    let config = &DuckmanConfig::load()?;
    let duckdb = find_duckdb_binary()?;
    let duckdb_version = config.get_duckdb_version(&None);
    let sql = if DUCKDB_CORE_EXTENSIONS.contains(&name) {
        format!("install {}", name)
    } else {
        format!("install {} from community", name)
    };
    println!(
        "Begin to install extension {} for DuckDB {}",
        name.green(),
        duckdb_version.unwrap_or("".to_owned())
    );
    let output = Command::new(&duckdb).args(["-c", &sql]).output();
    match output {
        Ok(out) => {
            if out.status.success() {
                let output = String::from_utf8_lossy(&out.stdout);
                if !output.trim().is_empty() {
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
    let config = DuckmanConfig::load()?;
    let version = config.get_duckdb_version(&None);
    if version.is_none() {
        anyhow::bail!("No duckdb version found!");
    }
    let path = DuckmanConfig::extension_path(&version.unwrap(), name);
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

// ── migrate ───────────────────────────────────────────────────────────────────

pub fn migrate_extensions(from_version: &str) -> anyhow::Result<()> {
    let from_version = normalize_duckdb_version(from_version);
    let old_binary = DuckmanConfig::version_binary(&from_version);
    if !old_binary.exists() {
        anyhow::bail!(
            "DuckDB {} is not installed (binary not found at {})",
            from_version,
            old_binary.display()
        );
    }

    // Query installed extensions from old version via CSV output (easy to parse)
    let output = Command::new(&old_binary)
        .args([
            "-csv",
            "-c",
            "SELECT extension_name FROM duckdb_extensions() WHERE installed = true",
        ])
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to query extensions from {}: {}",
            from_version,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    // Parse CSV: first line is header "extension_name", rest are names
    let stdout = String::from_utf8_lossy(&output.stdout);
    let extensions: Vec<&str> = stdout
        .lines()
        .skip(1)
        .map(|l| l.trim().trim_matches('"'))
        .filter(|l| !l.is_empty())
        .collect();

    if extensions.is_empty() {
        println!("No installed extensions found in DuckDB {}.", from_version);
        return Ok(());
    }

    println!(
        "Extensions installed in DuckDB {}: {}",
        from_version.yellow(),
        extensions.join(", ")
    );
    println!();

    // Target: current default duckdb
    let current_binary = find_duckdb_binary()?;
    let config = DuckmanConfig::load()?;
    let current_version = config
        .get_duckdb_version(&None)
        .unwrap_or_else(|| "current".to_string());
    println!("Installing into DuckDB {}...", current_version.green());

    let mut ok_count = 0usize;
    let mut fail_count = 0usize;

    for ext_name in &extensions {
        let sql = if DUCKDB_CORE_EXTENSIONS.contains(ext_name) {
            format!("install {}", ext_name)
        } else {
            format!("install {} from community", ext_name)
        };

        print!("  {:.<30} ", ext_name.green());
        let _ = std::io::stdout().flush();

        match Command::new(&current_binary).args(["-c", &sql]).output() {
            Ok(out) if out.status.success() => {
                println!("{}", "ok".green());
                ok_count += 1;
            }
            Ok(out) => {
                println!("{}", "failed".red());
                let stderr = String::from_utf8_lossy(&out.stderr);
                let msg = stderr.lines().next().unwrap_or("").trim();
                if !msg.is_empty() {
                    println!("    {}", msg.dimmed());
                }
                fail_count += 1;
            }
            Err(e) => {
                println!("{}", "error".red());
                println!("    {}", e.to_string().dimmed());
                fail_count += 1;
            }
        }
    }

    println!();
    println!(
        "Migration complete: {} installed, {} failed.",
        ok_count.to_string().green(),
        if fail_count > 0 {
            fail_count.to_string().red().to_string()
        } else {
            "0".to_string()
        }
    );
    Ok(())
}

// ── update ────────────────────────────────────────────────────────────────────

pub fn update_extensions() -> anyhow::Result<()> {
    let duckdb = find_duckdb_binary()?;
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
        install_extension(ext_name).await?;
        Ok(())
    }
}
