use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{env, fs};
use toml::Value;

pub const CORE_EXTENSIONS_CSV: &str = include_str!("resources/core_extensions.csv");
pub const COMMUNITY_EXTENSIONS_CSV: &str = include_str!("resources/community_extensions.csv");

pub const DUCKDB_CORE_EXTENSIONS: [&str; 28] = [
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

#[cfg(unix)]
fn binary_name() -> &'static str {
    "duckdb"
}

#[cfg(windows)]
fn binary_name() -> &'static str {
    "duckdb.exe"
}

lazy_static! {
    static ref EMPTY_PROFILES: HashMap<String, Profile> = HashMap::new();
}
pub fn duckman_home_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not find home directory")
        .join(".duckdb")
}

/// Top-level structure of ~/.duckdb/duckman-example.toml
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct DuckmanConfig {
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub profile: Option<HashMap<String, Profile>>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Profile {
    pub description: Option<String>,
    pub duckdb_version: Option<String>,
    pub parquet_key: Option<String>,
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub secret: HashMap<String, toml::Table>,
    #[serde(default)]
    pub bucket: HashMap<String, toml::Table>,
    #[serde(default)]
    pub attached: HashMap<String, AttachedDb>,
    #[serde(default)]
    pub ducklake: HashMap<String, DuckLake>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Secret {
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub secret_type: String,
    pub provider: Option<String>,
    pub key_id: Option<String>,
    pub secret: Option<String>,
    pub region: Option<String>,
    pub endpoint: Option<String>,
    pub account_name: Option<String>,
    pub connection_string: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachedDb {
    #[serde(rename = "type")]
    pub db_type: Option<String>,
    pub endpoint: String,
    pub encryption_key: Option<String>,
    pub sql: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuckLake {
    pub catalog_endpoint: String,
    pub data_path: String,
    pub sql: Option<String>,
}

impl DuckmanConfig {
    pub fn home_dir() -> PathBuf {
        duckman_home_dir()
    }

    pub fn versions_dir() -> PathBuf {
        Self::home_dir().join("versions")
    }

    pub fn version_dir(version: &str) -> PathBuf {
        Self::versions_dir().join(normalize_duckdb_version(version))
    }

    pub fn version_binary(version: &str) -> PathBuf {
        Self::version_dir(version).join(binary_name())
    }

    pub fn extension_path(duckdb_version: &str, ext_name: &str) -> PathBuf {
        DuckmanConfig::home_dir()
            .join("extensions")
            .join(duckdb_version)
            .join(duckdb_platform_id())
            .join(format!("{}.duckdb_extension", ext_name))
    }

    pub fn config_file() -> PathBuf {
        Self::home_dir().join("duckman.toml")
    }

    pub fn installed_versions() -> Vec<String> {
        let versions_dir = Self::versions_dir();
        if !versions_dir.exists() {
            return Vec::new();
        }
        let mut versions: Vec<String> = fs::read_dir(&versions_dir)
            .map(|rd| {
                rd.filter_map(|e| {
                    let entry = e.ok()?;
                    if entry.path().is_dir() {
                        Some(entry.file_name().to_string_lossy().to_string())
                    } else {
                        None
                    }
                })
                .collect()
            })
            .unwrap_or_default();
        versions.sort();
        versions
    }

    pub fn is_duckdb_installed(duckdb_version: &str) -> bool {
        Self::version_binary(duckdb_version).exists()
    }

    pub fn is_ext_installed(duckdb_version: &str, ext_name: &str) -> bool {
        let buf = Self::extension_path(duckdb_version, ext_name);
        buf.exists()
    }

    pub fn load() -> anyhow::Result<Self> {
        let config_file = Self::config_file();
        Self::load_from(config_file)
    }

    pub fn load_from<P: AsRef<Path>>(config_file: P) -> anyhow::Result<Self> {
        if config_file.as_ref().exists() {
            let content = fs::read_to_string(&config_file)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(DuckmanConfig {
                ..Default::default()
            })
        }
    }

    pub fn get_duckdb_version(&self, profile_name: &Option<String>) -> Option<String> {
        if let Ok(version) = env::var("DUCKDB_VERSION") {
            return Some(version);
        }
        if let Some(profile_name) = profile_name {
            if let Some(profiles) = &self.profile {
                if let Some(selected_profile) = profiles.get(profile_name) {
                    if let Some(profile_duckdb_version) = &selected_profile.duckdb_version {
                        Some(profile_duckdb_version.to_string());
                    }
                }
            }
        }
        if self.default.is_some() {
            return Some(self.default.clone().unwrap());
        }
        let versions = Self::installed_versions();
        if let Some(version) = versions.into_iter().next() {
            return Some(version);
        }
        None
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let home = Self::home_dir();
        fs::create_dir_all(&home)?;
        let content = toml::to_string_pretty(self)?;
        fs::write(Self::config_file(), content)?;
        Ok(())
    }

    pub fn set_default(&mut self, version: &str) {
        let duckdb_version = normalize_duckdb_version(version);
        self.default = Some(duckdb_version);
    }

    pub fn get_profiles(&self) -> &HashMap<String, Profile> {
        if self.profile.is_some() {
            return self.profile.as_ref().unwrap();
        }
        &EMPTY_PROFILES
    }
}

pub fn convert_secret_to_sql(name: &str, secret_value: &toml::Table) -> String {
    if let Some(sql) = secret_value.get("sql") {
        // replace \n with space and convert to one-line string
        return sql.as_str().unwrap().trim().replace('\n', " ");
    }
    let mut sql = format!("CREATE OR REPLACE SECRET {} (", name);
    for (key, value) in secret_value {
        sql.push_str(&format!(
            " {} {},",
            key,
            convert_toml_value_to_sql_value(value)
        ));
    }
    sql.remove(sql.len() - 1);
    //sql = sql[0..sql.len() - 2].to_string();
    sql.push_str(");");
    sql
}

pub fn convert_bucket_to_sql(name: &str, bucket: &toml::Table) -> String {
    if let Some(sql) = bucket.get("sql") {
        // replace \n with space and convert to one-line string
        return sql.as_str().unwrap().trim().replace('\n', " ");
    }
    let mut sql = format!("CREATE OR REPLACE SECRET {} (", name);
    for (key, value) in bucket {
        let mut value = convert_toml_value_to_sql_value(value);
        if key.eq_ignore_ascii_case("type") || key.eq_ignore_ascii_case("provider") {
            value = value.trim_matches('\'').to_string();
        }
        sql.push_str(&format!(" {} {},", key, value));
    }
    sql.remove(sql.len() - 1);
    //sql = sql[0..sql.len() - 2].to_string();
    sql.push_str(");");
    sql
}

pub fn convert_attached_db_to_sql(name: &str, db: &AttachedDb) -> String {
    if let Some(sql) = db.sql.as_ref() {
        // replace \n with space and convert to one-line string
        return sql.trim().replace('\n', " ");
    }
    if let Some(encryption_key) = &db.encryption_key {
        format!(
            "ATTACH '{}' AS {} ( ENCRYPTION_KEY '{}');",
            db.endpoint, name, encryption_key
        )
    } else if let Some(db_type) = &db.db_type {
        format!("ATTACH '{}' AS {} ( type {});", db.endpoint, name, db_type)
    } else {
        // such as motherduck, `md:xxx`
        format!("ATTACH '{}' AS {};", db.endpoint, name)
    }
}

pub fn convert_ducklake_to_sql(name: &str, db: &DuckLake) -> String {
    if let Some(sql) = db.sql.as_ref() {
        // replace \n with space and convert to one-line string
        return sql.trim().replace('\n', " ");
    }
    format!(
        "ATTACH  '{}'  as {} (DATA_PATH '{}');",
        db.catalog_endpoint, name, db.data_path
    )
}

fn convert_toml_value_to_sql_value(value: &toml::Value) -> String {
    match value {
        Value::String(s) => {
            format!("'{}\'", s.as_str())
        }
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Datetime(d) => d.to_string(),
        Value::Array(a) => a
            .iter()
            .map(|v| convert_toml_value_to_sql_value(v))
            .collect::<Vec<String>>()
            .join(", "),
        Value::Table(t) => {
            let pairs = t
                .iter()
                .map(|(k, v)| format!("'{}': {}", k, convert_toml_value_to_sql_value(v)))
                .collect::<Vec<String>>()
                .join(", ");
            format!("MAP {{{}}}", pairs)
        }
    }
}

pub fn inject_profile(
    duckdb_version: &str,
    profile: &Profile,
    args: &mut Vec<String>,
    new_env: &mut HashMap<String, String>,
) {
    // load or install extensions
    for ext_name in profile.extensions.iter() {
        if !DuckmanConfig::is_ext_installed(duckdb_version, ext_name) {
            let sql = if DUCKDB_CORE_EXTENSIONS.contains(&ext_name.as_ref()) {
                format!("install {};", ext_name)
            } else {
                format!("install {} from community;", ext_name)
            };
            args.push("-cmd".to_owned());
            args.push(sql);
        } else {
            args.push("-cmd".to_owned());
            args.push(format!("load {};", ext_name));
        }
    }
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
    // secrets
    for (name, value) in profile.secret.iter() {
        let sql = convert_secret_to_sql(name, value);
        args.push("-cmd".to_owned());
        args.push(sql);
    }
    // buckets
    for (name, value) in profile.bucket.iter() {
        let sql = convert_bucket_to_sql(name, value);
        args.push("-cmd".to_owned());
        args.push(sql);
    }
    // parquet key
    if let Some(parquet_key) = &profile.parquet_key {
        let sql = format!("PRAGMA add_parquet_key('key256','{}');", parquet_key);
        args.push("-cmd".to_owned());
        args.push(sql);
    }
    // attached databases
    for (name, attached_db) in profile.attached.iter() {
        let sql = convert_attached_db_to_sql(name, attached_db);
        args.push("-cmd".to_owned());
        args.push(sql);
    }
    // ducklake
    for (name, ducklake) in profile.ducklake.iter() {
        let sql = convert_ducklake_to_sql(name, ducklake);
        args.push("-cmd".to_owned());
        args.push(sql);
    }
}

pub fn normalize_duckdb_version(version: &str) -> String {
    if version.starts_with('v') {
        version.to_string()
    } else {
        format!("v{}", version)
    }
}

fn duckdb_platform_id() -> &'static str {
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
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::stdout;
    use testresult::TestResult;

    #[test]
    fn test_load_from() -> TestResult {
        let config = DuckmanConfig::load_from("duckman-example.toml")?;
        println!("{:?}", config);
        Ok(())
    }

    #[test]
    fn test_load_profiles() -> TestResult {
        let config = DuckmanConfig::load_from("duckman-example.toml")?;
        for entry in config.get_profiles() {
            println!("{}", entry.0);
            println!("{:?}", entry.1);
        }
        Ok(())
    }

    #[test]
    fn test_secrets() -> TestResult {
        let config = DuckmanConfig::load_from("duckman-example.toml")?;
        let default_profile = config.get_profiles().get("default").unwrap();
        for (key, value) in default_profile.secret.iter() {
            println!("{}", key);
            println!("sql: {:?}", convert_secret_to_sql("hello", value));
        }
        Ok(())
    }

    #[test]
    fn test_buckets() -> TestResult {
        let config = DuckmanConfig::load_from("duckman-example.toml")?;
        let default_profile = config.get_profiles().get("default").unwrap();
        for (key, value) in default_profile.bucket.iter() {
            println!("{}", key);
            println!("sql: {:?}", convert_bucket_to_sql(key, value));
        }
        Ok(())
    }

    #[test]
    fn test_convert_secret_to_sql() -> TestResult {
        Ok(())
    }

    #[test]
    fn test_ext_installed() -> TestResult {
        let installed = DuckmanConfig::is_ext_installed("v1.5.2", "shellfs");
        println!("installed: {}", installed);
        Ok(())
    }
}
