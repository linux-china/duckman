use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use toml::Value;

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

/// Top-level structure of ~/.duckdb/duckman.toml
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
    #[serde(default)]
    pub extensions: Vec<String>,
    #[serde(default)]
    pub environments: HashMap<String, String>,
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
    pub db_type: String,
    pub endpoint: String,
    pub encryption_key: Option<String>,
    pub sql: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuckLake {
    pub catalog_endpoint: String,
    pub data_path: String,
}

impl DuckmanConfig {
    pub fn home_dir() -> PathBuf {
        duckman_home_dir()
    }

    pub fn versions_dir() -> PathBuf {
        Self::home_dir().join("versions")
    }

    pub fn version_dir(version: &str) -> PathBuf {
        Self::versions_dir().join(version)
    }

    pub fn version_binary(version: &str) -> PathBuf {
        Self::version_dir(version).join(binary_name())
    }

    pub fn config_file() -> PathBuf {
        Self::home_dir().join("duckman.toml")
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

    pub fn save(&self) -> anyhow::Result<()> {
        let home = Self::home_dir();
        fs::create_dir_all(&home)?;
        let content = toml::to_string_pretty(self)?;
        fs::write(Self::config_file(), content)?;
        Ok(())
    }

    pub fn is_installed(&self, version: &str) -> bool {
        Self::version_binary(version).exists()
    }

    pub fn installed_versions(&self) -> Vec<String> {
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

    pub fn set_default(&mut self, version: &str) {
        self.default = Some(version.to_string());
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
    let mut sql = format!("CREATE SECRET {} (", name);
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
    let mut sql = format!("CREATE create {} (", name);
    for (key, value) in bucket {
        let mut value = convert_toml_value_to_sql_value(value);
        if key.eq_ignore_ascii_case("type") {
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
    format!(
        "ATTACH '{}' AS {} ( type {});",
        db.endpoint, name, db.db_type
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

#[cfg(test)]
mod tests {
    use super::*;
    use testresult::TestResult;

    #[test]
    fn test_load_from() -> TestResult {
        let config = DuckmanConfig::load_from("duckman.toml")?;
        Ok(())
    }

    #[test]
    fn test_load_profiles() -> TestResult {
        let config = DuckmanConfig::load_from("duckman.toml")?;
        for entry in config.get_profiles() {
            println!("{}", entry.0);
            println!("{:?}", entry.1);
        }
        Ok(())
    }

    #[test]
    fn test_secrets() -> TestResult {
        let config = DuckmanConfig::load_from("duckman.toml")?;
        let default_profile = config.get_profiles().get("default").unwrap();
        for (key, value) in default_profile.secret.iter() {
            println!("{}", key);
            println!("sql: {:?}", convert_secret_to_sql("hello", value));
        }
        Ok(())
    }

    #[test]
    fn test_buckets() -> TestResult {
        let config = DuckmanConfig::load_from("duckman.toml")?;
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
}
