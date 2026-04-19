use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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
    pub secrets: Vec<toml::Table>,
    #[serde(default)]
    pub s3_buckets: Vec<S3Bucket>,
    #[serde(default)]
    pub attached: Vec<AttachedDb>,
    #[serde(default)]
    pub ducklakes: Vec<DuckLake>,
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
pub struct S3Bucket {
    pub name: String,
    pub access_key: String,
    pub secret_key: String,
    pub endpoint: Option<String>,
    pub region: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttachedDb {
    pub name: String,
    #[serde(rename = "type")]
    pub db_type: String,
    pub endpoint: String,
    pub encryption_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuckLake {
    pub name: String,
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
}
