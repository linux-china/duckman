use crate::duckman_config::DuckmanConfig;
use colored::Colorize;

// ── Commands ──────────────────────────────────────────────────────────────────

pub fn list_profiles() -> anyhow::Result<()> {
    let duckman_config = DuckmanConfig::load()?;
    let profiles = duckman_config.get_profiles();

    if profiles.is_empty() {
        println!("No profiles configured.");
        println!();
        println!("Example:");
        println!("  [profile.default]");
        println!("  description = \"My default profile\"");
        println!("  extensions = [\"json\", \"parquet\", \"httpfs\"]");
        return Ok(());
    }

    for (name, profile) in profiles {
        // ── header ────────────────────────────────────────────────────────────
        print!("  {}", name.green().bold());
        if let Some(desc) = &profile.description {
            print!("  {}", desc.dimmed());
        }
        println!();

        // ── duckdb version ────────────────────────────────────────────────────
        if let Some(ver) = &profile.duckdb_version {
            println!("    duckdb:      {}", ver);
        }

        // ── extensions ────────────────────────────────────────────────────────
        if !profile.extensions.is_empty() {
            println!("    extensions:  {}", profile.extensions.join(", "));
        }

        // ── environments ──────────────────────────────────────────────────────
        if !profile.environment.is_empty() {
            let vars: Vec<String> = profile
                .environment
                .iter()
                .map(|(k, v)| format!("{}={}", k.to_uppercase(), v))
                .collect();
            println!("    env:         {}", vars.join("  "));
        }

        // ── secrets ───────────────────────────────────────────────────────────
        if !profile.secret.is_empty() {
            let labels: Vec<String> = profile
                .secret
                .iter()
                .map(|(name, value)| {
                    let secret_type = if let Some(name) = value.get("type") {
                        name.as_str().unwrap().to_string()
                    } else {
                        "(unknown)".to_owned()
                    };
                    format!("{} [{}]", name, secret_type)
                })
                .collect();
            println!("    secrets:     {}", labels.join(", "));
        }

        // ── storage buckets ────────────────────────────────────────────────────────
        if !profile.bucket.is_empty() {
            let names: Vec<&str> = profile
                .bucket
                .iter()
                .map(|(name, value)| name.as_str())
                .collect();
            println!("    storage buckets:  {}", names.join(", "));
        }

        // ── attached DBs ──────────────────────────────────────────────────────
        if !profile.attached.is_empty() {
            for (name, db) in &profile.attached {
                println!(
                    "    attached:    {} [{}] {}",
                    name,
                    db.db_type.clone().unwrap_or("".to_string()),
                    db.endpoint.dimmed()
                );
            }
        }

        // ── ducklake ─────────────────────────────────────────────────────────
        if !profile.ducklake.is_empty() {
            for (name, lake) in &profile.ducklake {
                println!(
                    "    ducklake:    {} → {}",
                    name,
                    lake.catalog_endpoint.dimmed()
                );
            }
        }

        println!();
    }

    Ok(())
}
