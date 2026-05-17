use crate::duckman_config::{
    DUCKDB_CORE_EXTENSIONS, DuckmanConfig, convert_attached_db_to_sql, convert_bucket_to_sql,
    convert_ducklake_to_sql, convert_secret_to_sql, decrypt_value,
};
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
        let scope = if let Some(scope) = &profile.scope {
            format!("({})", scope)
        } else {
            "".to_string()
        };
        print!("  {}{}", name.green().bold(), scope.cyan().bold());
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
                    db.db_path.dimmed()
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

pub fn dump_profile(profile_name: &str) -> anyhow::Result<()> {
    let config = DuckmanConfig::load()?;
    let profiles = config.get_profiles();

    let profile = profiles
        .get(profile_name)
        .ok_or_else(|| anyhow::anyhow!("Profile '{}' not found.", profile_name))?;

    // duckdb bin executable path
    let mut duckdb_bin = if let Some(ver) = &profile.duckdb_version {
        DuckmanConfig::version_binary(ver)
            .to_string_lossy()
            .to_string()
    } else if let Some(ver) = config.default.as_deref() {
        DuckmanConfig::version_binary(ver)
            .to_string_lossy()
            .to_string()
    } else {
        "duckdb".to_string()
    };
    if let Some(user_home_path) = dirs::home_dir() {
        let user_home = user_home_path.to_string_lossy().to_string();
        if duckdb_bin.starts_with(&user_home) {
            // replace user home with `$HOME`
            duckdb_bin = duckdb_bin.replace(&user_home, "$HOME");
        }
    }

    println!("#!/bin/sh");
    if let Some(desc) = &profile.description {
        println!("# Profile: {} - {}", profile_name, desc);
    } else {
        println!("# Profile: {}", profile_name);
    }

    // private key
    let private_key = &config.get_private_key();

    // environment variables
    if !profile.environment.is_empty() {
        println!("# Environment variables:");
        for (key, value) in &profile.environment {
            println!(
                "export {}={}",
                key.to_uppercase(),
                shell_escape(&decrypt_value(private_key, value))
            );
        }
        println!();
    }
    // extensions install
    if !profile.extensions.is_empty() {
        println!("# Install extensions:");
        let mut command_line = duckdb_bin.clone();
        for ext_name in &profile.extensions {
            if DUCKDB_CORE_EXTENSIONS.contains(&ext_name.as_ref()) {
                command_line.push_str(&format!(" -c 'install {}'", ext_name));
            } else {
                command_line.push_str(&format!(" -c 'install {} from community'", ext_name));
            };
        }
        println!("{}", command_line);
    }

    // collect -cmd arguments
    let mut cmds: Vec<String> = Vec::new();

    for ext in &profile.extensions {
        cmds.push(format!("load {};", ext));
    }
    if let Some(parquet_key) = &profile.parquet_key {
        cmds.push(format!(
            "PRAGMA add_parquet_key('key256','{}');",
            parquet_key
        ));
    }
    for (name, value) in &profile.secret {
        cmds.push(convert_secret_to_sql(private_key, name, value));
    }
    for (name, value) in &profile.bucket {
        cmds.push(convert_bucket_to_sql(private_key, name, value));
    }
    for (name, db) in &profile.attached {
        cmds.push(convert_attached_db_to_sql(private_key, name, db));
    }
    for (name, lake) in &profile.ducklake {
        cmds.push(convert_ducklake_to_sql(name, lake));
    }

    if !cmds.is_empty() {
        println!();
        print!("{}", duckdb_bin);
        for cmd in &cmds {
            print!(" \\\n  -cmd {}", shell_escape(cmd));
        }
        println!(" \\\n  \"$@\"");
    }

    Ok(())
}

fn shell_escape(s: &str) -> String {
    if s.contains(|c: char| {
        matches!(
            c,
            ' ' | '\t'
                | '"'
                | '\''
                | '\\'
                | '$'
                | '`'
                | '!'
                | '('
                | ')'
                | '{'
                | '}'
                | '|'
                | '&'
                | ';'
        )
    }) {
        format!("'{}'", s.replace('\'', "'\\''"))
    } else {
        s.to_string()
    }
}
