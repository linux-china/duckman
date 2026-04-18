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
        if !profile.environments.is_empty() {
            let vars: Vec<String> = profile
                .environments
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            println!("    env:         {}", vars.join("  "));
        }

        // ── secrets ───────────────────────────────────────────────────────────
        if !profile.secrets.is_empty() {
            let labels: Vec<String> = profile
                .secrets
                .iter()
                .map(|s| {
                    let label = s.name.as_deref().unwrap_or("(unnamed)");
                    format!("{} [{}]", label, s.secret_type)
                })
                .collect();
            println!("    secrets:     {}", labels.join(", "));
        }

        // ── S3 buckets ────────────────────────────────────────────────────────
        if !profile.s3_buckets.is_empty() {
            let names: Vec<&str> = profile.s3_buckets.iter().map(|b| b.name.as_str()).collect();
            println!("    s3_buckets:  {}", names.join(", "));
        }

        // ── attached DBs ──────────────────────────────────────────────────────
        if !profile.attached.is_empty() {
            for db in &profile.attached {
                println!(
                    "    attached:    {} [{}] {}",
                    db.name,
                    db.db_type,
                    db.endpoint.dimmed()
                );
            }
        }

        // ── ducklakes ─────────────────────────────────────────────────────────
        if !profile.ducklakes.is_empty() {
            for lake in &profile.ducklakes {
                println!(
                    "    ducklake:    {} → {}",
                    lake.name,
                    lake.catalog_endpoint.dimmed()
                );
            }
        }

        println!();
    }

    Ok(())
}
